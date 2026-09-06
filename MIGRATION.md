# Migrating from 0.3 to 0.4

This is a breaking API refactor, targeting Rust 1.88 and edition 2024.

| 0.3 | 0.4 |
| --- | --- |
| `Fn(State, Action) -> State` reducer | `Fn(State, &Action) -> State` |
| `Store::new(reducer)` requires Tokio | Synchronous, runtime-independent construction |
| `store.dispatch(action).await` | `store.dispatch(action)?` returns the action or middleware's output |
| `store.select(selector).await` | `store.select(selector)`; `try_select` for explicit error handling |
| `store.state_cloned().await` | `store.get_state()` or `store.state_cloned()` |
| `store.wrap(layer).await` | `Store::builder(reducer).wrap(layer).build()` |
| `MiddleWare`, `StoreWithMiddleware`, `StoreApi` trait | `Middleware`, inferred builder/store types, weak `MiddlewareApi` handle |
| Middleware receives `&Arc<Inner>` | Receives `api` for full-chain dispatch and `next` for forwarding |
| `subscribe(callback).await` returns nothing | Retain a `Subscription`; unsubscribe explicitly or on drop |
| State callback borrows the worker's state | `subscribe` supplies a cloned snapshot; `listen` requires no clone |
| Async `Thunk` trait and `ActionOrThunk` enum | `thunk(FnOnce)` returning an awaitable result |
| Thunks launch detached Tokio tasks | Caller polls/awaits their result; cancellation is explicit |
| Store handle shared across tasks/threads | Local synchronous `Store`; optional `AsyncStore` for remote dispatch |
| `middleware_logger`, `middleware_thunk` features | `logger`, `thunk` |

For action-preserving closures, use `.middleware(|api, next, action| ...)`.
For a distinct new input, use `.wrap(middleware(|api, next, input: NewInput| ...))`
and implement `From<PreviousInput> for NewInput`. The builder promotes older inputs
through all subsequent wrappers; no transitive `From` implementations are needed.
Each introduced type must be distinct to keep compile-time routing unambiguous.
The old `init` hook is gone: prepare middleware resources before wrapping, then
dispatch startup actions through the complete store after `build`.

Middleware closures return `DispatchResult<T>` and can change `T`. Async thunks
return `Result<T, E>` with `E: From<DispatchError>`. Thunk input functions themselves
are not sent to action-only middleware; their emitted actions enter the complete
chain, even if the action middleware was added after `ThunkMiddleware`.

The old worker/message traits and detached-thunk trait example have been removed.
The optional Tokio adapter is a bounded mailbox with explicit shutdown instead.
Do not await that mailbox from within synchronous store callbacks; use the local
`MiddlewareApi` to dispatch directly. Reducers remain pure and synchronous.
