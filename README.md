# redux-rs

A typed Redux-style store for Rust. This branch contains the **breaking, unreleased
0.4 API**; see [MIGRATION.md](MIGRATION.md) when upgrading from 0.3.

The default build has no dependencies. It requires Rust 1.88 or newer.

## A synchronous store

```rust
use redux_rs::{DispatchError, Store};

#[derive(Debug, PartialEq)]
enum Action { Increment }

fn reducer(state: i32, action: &Action) -> i32 {
    match action { Action::Increment => state + 1 }
}

fn main() -> Result<(), DispatchError> {
    let store = Store::new(reducer);
    let subscription = store.subscribe(|count: &i32| println!("Count: {count}"));

    assert_eq!(store.dispatch(Action::Increment)?, Action::Increment);
    assert_eq!(store.get_state(), 1);

    subscription.unsubscribe(); // Also happens when the handle is dropped.
    Ok(())
}
```

Reducers own the previous state and borrow the action. No `Clone` bound is needed
for either state or actions to dispatch. `select` reads a portion of state without
cloning, and accepts closures that borrow local variables or consume captures.
Use `Store::new_with_state` or `Store::builder_with_state` for non-default states.

## Middleware

Use `.middleware(...)` for a closure that preserves the current action type:

```rust
let store = Store::builder(reducer)
    .middleware(|api, next, action| {
        println!("Before: {action:?}");
        let result = next.dispatch(action);
        println!("After: {}", api.get_state());
        result
    })
    .build();
```

`next.dispatch(action)` continues through the remaining layers. Calling
`api.dispatch(action)` starts again at the outermost layer. Middleware can
transform, reject, swallow, or multiply actions, and change the typed result.
It returns `DispatchResult<Output>`; rejection uses `DispatchError::Rejected`.
`next` is cloneable and can be retained for delayed forwarding. It holds a weak
connection to state; forwarding after the store is dropped returns `StoreDropped`.
Reduction and notification finish before dispatch returns only when middleware
forwards synchronously. A successful dispatch may instead have swallowed or
deferred the action; it does not guarantee a state update.

The last wrapper runs first: `builder.wrap(a).wrap(b)` dispatches through
`b -> a -> reducer`, then unwinds through `a -> b`. Only the completed builder
exposes a store; there is no partially initialized dispatcher.
Prepare middleware resources before wrapping; dispatch startup actions after `build`.

### Additional action types

Use `.wrap(middleware(...))` to introduce a **distinct new input type**, with
`From<PreviousInput>` defining how older actions enter it:

```rust
use redux_rs::middleware;

struct ActionWithLogLevel { action: Action, level: u8 }

impl From<Action> for ActionWithLogLevel {
    fn from(action: Action) -> Self { Self { action, level: 1 } }
}

let store = Store::builder(reducer)
    .wrap(middleware(|_, next, input: ActionWithLogLevel| {
        println!("Level {}: {:?}", input.level, input.action);
        next.dispatch(input.action)
    }))
    .build();

store.dispatch(Action::Increment)?;
store.dispatch(ActionWithLogLevel { action: Action::Increment, level: 3 })?;
```

Each wrapper adds its input to the set of accepted types. If a subsequent wrapper
adds `TracedAction: From<ActionWithLogLevel>`, all three types remain directly
dispatchable. The crate composes the conversions before entering the complete
chain; callers do not construct nested envelopes. Unsupported inputs fail to compile.
Use `.middleware(...)` when preserving the current input type, so routing
remains unambiguous. Built-in middleware preserves input types automatically.

## Awaitable thunks

Enable the `thunk` feature and add `ThunkMiddleware` to the builder:

```rust
use redux_rs::middlewares::thunk::{thunk, ThunkMiddleware};

let store = Store::builder(reducer).wrap(ThunkMiddleware).build();
let count = store.dispatch(thunk(|api| async move {
    api.dispatch(Action::Increment)?;
    Ok::<_, DispatchError>(api.get_state())
})).await?;
```

Thunks accept `FnOnce` closures, can borrow caller-owned data, return application
values and errors, and dispatch nested thunks. Their error type implements
`From<DispatchError>`. No task is spawned automatically. When a call reaches
`ThunkMiddleware`, it invokes the closure to obtain its future; polling the returned
future runs the asynchronous body. An interceptor may reject or delay that call.
Dropping the future cancels remaining work; actions already dispatched stay applied.
The future retains the store until completion or cancellation.

Thunk calls follow middleware order and stop at `ThunkMiddleware`, just as Redux
thunks stop at their handler. Implement `Middleware::dispatch_thunk`, or add a
`.thunk_middleware(...)` closure **after** `.wrap(ThunkMiddleware)` to intercept them:

```rust
let store = Store::builder(reducer)
    .wrap(ThunkMiddleware)
    .thunk_middleware(|api, next, call| {
        if api.get_state() < 0 {
            return Err(DispatchError::Rejected("thunks disabled".into()));
        }
        let future = next.dispatch_thunk(call)?;
        Ok(Box::pin(async move {
            let outcome = future.await?;
            println!("Thunk finished: {outcome:?}");
            Ok(outcome)
        }))
    })
    .build();
```

Action-only closures forward thunk calls unchanged; thunk-only closures forward
actions unchanged. `LoggerMiddleware` logs both when placed outside the thunk handler.
Interceptors see `ThunkOutcome::Succeeded` or `Failed`; the original application
value/error, even one borrowing caller data, stays typed for the caller. Interceptors
can return a `DispatchError` before or after execution, but cannot fabricate an
arbitrary typed result. Success without running the task is reported as `Rejected`.

All actions emitted by a thunk restart at the outermost layer, including wrappers
added after `ThunkMiddleware`. Thunks are executor independent; with Tokio, use
local tasks rather than `tokio::spawn` for these non-`Send` store handles.

See [typed_middleware.rs](examples/typed_middleware.rs) for a complete runnable
example combining ordinary actions, log-level actions, and nested thunks:

```sh
cargo run --example typed_middleware --features logger,thunk
```

## Subscriptions and lifetime

`listen(|| ...)` provides Redux-style notifications without cloning state. Read
state through a captured `store.api()` handle. `subscribe(|state: &State| ...)`
is a convenience requiring `State: Clone`; each callback gets a snapshot so its
reference remains valid across nested dispatches. Keep the returned `Subscription`.

Both forms notify after every action that reaches the reducer, including no-op
actions. Listener order follows registration order. The listener list is snapshotted
per dispatch: additions/removals affect the next dispatch, including nested dispatches.
Each state subscriber reads the latest state when it runs, so earlier listeners'
nested updates are visible to subsequent listeners.

Cloned stores share ownership. `MiddlewareApi` and subscription handles are weak;
they do not keep a store alive. Prefer capturing `store.api()` in callbacks to avoid
ownership cycles. Dispatch through an expired API returns `StoreDropped`.
Retained `Next` handles do not retain store state, but do retain downstream
middleware and reducer captures until those handles are dropped.

Reducers cannot read or dispatch through the store; attempts return `Reducing`.
An action reaching the reducer during a selector's state borrow returns
`StateBorrowed`. Middleware runs first and may have side effects, swallow the action,
or defer reduction until the borrow ends. Application panics unwind normally.
A reducer panic poisons the store because its previous
state was moved into the reducer; subsequent reduction/read attempts return
`Poisoned`. `select`/`get_state` panic on unavailable state; use `try_select` to
handle those errors explicitly.

## Optional Tokio mailbox

Enable `tokio` for `store.into_async(capacity)`. Create it inside a Tokio `LocalSet`.
The store stays local while cloned `AsyncStore` handles send work from other tasks
or threads. The bounded mailbox provides backpressure. Both `dispatch` and `select`
return awaitable results; action values and selection closures sent across the
mailbox must be `Send`, as must their returned values.

Once enqueued, work runs even if the requesting future is dropped. Dropping the last
handle drains the mailbox and releases the worker's store. `shutdown().await` closes
all handles and waits for queued work to finish. Worker failures return `WorkerStopped`.
The mailbox accepts ordinary action inputs; dispatch asynchronous thunks on the local
store directly. Keep a synchronous clone if both interfaces are needed.
Generic dispatch helpers can use the public `Accepted: Promote<Input, Path>` bound;
the compiler infers `Path` for each supported input type.

## Features and checks

| Feature | Enables |
| --- | --- |
| `logger` | `LoggerMiddleware`, using the application's `log` setup |
| `thunk` | Executor-independent asynchronous thunk dispatch |
| `tokio` | Bounded asynchronous mailbox |

```sh
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```
