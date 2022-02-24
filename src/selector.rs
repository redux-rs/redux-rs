/// # Selector trait
/// Selectors are the way to get the current state and transform it into something useful for our app.
/// You can write a selector by implementing the `Selector` trait or by writing a function with the signature `Fn(&State) -> Result`
///
/// ## Trait example
/// ```
/// use redux_rs::Selector;
///
/// enum State {
///     Authorized { bearer_token: String },
///     Unauthorized
/// }
///
/// struct BearerTokenSelector;
/// impl Selector<State> for BearerTokenSelector {
///     type Result = Option<String>;
///
///     fn select(&self, state: &State) -> Self::Result {
///         match state {
///             State::Authorized { bearer_token } => Some(bearer_token.clone()),
///             State::Unauthorized => None
///         }
///     }
/// }
///
/// let selector = BearerTokenSelector;
/// let state = State::Authorized { bearer_token: "secret".to_string() };
/// assert_eq!(selector.select(&state), Some("secret".to_string()));
/// ```
///
/// ## Fn example
/// ```
/// use redux_rs::Selector;
///
/// enum State {
///     Authorized { bearer_token: String },
///     Unauthorized
/// }
///
/// struct BearerTokenSelector;
/// impl Selector<State> for BearerTokenSelector {
///     type Result = Option<String>;
///
///     fn select(&self, state: &State) -> Self::Result {
///         match state {
///             State::Authorized { bearer_token } => Some(bearer_token.clone()),
///             State::Unauthorized => None
///         }
///     }
/// }
///
/// let selector = |state: &State| {
///     match state {
///         State::Authorized { bearer_token } => Some(bearer_token.clone()),
///         State::Unauthorized => None
///     }
/// };
/// let state = State::Authorized { bearer_token: "secret".to_string() };
/// assert_eq!(selector.select(&state), Some("secret".to_string()));
/// ```
pub trait Selector<State> {
    type Result;

    fn select(&self, state: &State) -> Self::Result;
}

impl<F, State, Result> Selector<State> for F
where
    F: Fn(&State) -> Result,
{
    type Result = Result;

    fn select(&self, state: &State) -> Self::Result {
        self(state)
    }
}
