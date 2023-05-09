use super::effect::Effect;
use super::types::{EnvironmentType, MessageContext};

pub trait Reducer {
    /// A reducer can be messaged from a parent.
    /// `Messages` are send parent to child
    type Message: Clone;

    /// This type is used to delegate from a Child Reducer
    /// back to its parent. The parent can then decide whether
    /// to consume this event or whether to ignore it.
    /// DelegateMessages are send child to parent
    type DelegateMessage: Clone;

    /// The action is the internal type of the Reducer. It cannot
    /// be called or accessed by the outside
    type Action: std::fmt::Debug + Clone + Send;

    /// The state that this reducer can act upon
    type State;

    // The environment type we're using
    type Environment: EnvironmentType;

    fn reduce<'a, 'b>(
        context: &'a impl MessageContext<Self::Action, Self::DelegateMessage, Self::Message>,
        action: Self::Action,
        state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> Effect<'b, Self::Action>;

    /// Define the initial action when the reducer starts up
    fn initial_action() -> Option<Self::Action>;
}

/// A child reducer has two additional requirements. Converting types back and forth via Message
/// and DelegateMessage.
pub trait ChildReducer<Parent: Reducer>: Reducer {
    fn to_child(message: <Parent as Reducer>::Message) -> Option<<Self as Reducer>::Action>;

    fn from_child(message: <Self as Reducer>::DelegateMessage) -> Option<Parent::Action>;
}
