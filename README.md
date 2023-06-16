# Navicula

This is a simplified implementation of the [SwiftUI Composable Architecture (TCA)](https://github.com/pointfreeco/swift-composable-architecture) for the [Rust Dioxus Library](https://dioxuslabs.com). There're many similarities to the Elm architecture as well as various React Redux models. This is currently being used for the [Ebou Mastodon App](https://terhech.de/ebou). It is an early alpha.

## Features

- Reducers
- Actions
- Environment
- Nesting & Combining Child Reducers
- Sending messages to child reducers or from child reducers to the parent reducers
- View Stores which handle the View State

There's a super simple example in the `examples` folder to see what it does. You can run it via:

``` sh
cargo run --example gnarl
```

## Description

Navicula splits up code into

- State: All your view / logic state
- Reducer: All the logic
- Action: The actions that can be performed in a reducer
- View: The view. Any action here is sent to the reducer and the reducer and then the view is rerendered

The reducer expects that each handled action returns an `Effect`. There're multiple Effect types:

- Execute a Future
- Execute another Action
- Execute an Effect with some delay
- Run a timer that continously polls the reducer
- Merge multiple Effects together
- Perform javascript in the UI context

Here's an example of a simple reducer function:

``` rust

pub enum Action {
    Initial,
    Close,
    Chats(usize),
    Edit(Message),
    Edit2(Message),
    FinishEdit,
}

fn reduce<'a, 'b>(
    context: &'a impl MessageContext<Self::Action, Self::DelegateMessage, Self::Message>,
    action: Self::Action,
    _state: &'a mut Self::State,
    environment: &'a Self::Environment,
) -> Effect<'b, Self::Action> {
    match action {
        Action::Initial => {
            // fake subscription, just to see if drop works
            return environment
                .chats
                .subscribe("chat-chats", context, |data| Action::Chats(data.len()));
        }
        Action::Chats(cnt) => {
            log::info!("Have {cnt} chats");
        }
        Action::Edit(message) => {
            return Effect::action(Action::Edit2(message)).delay(Duration::from_secs(2))
        }
        Action::Edit2(message) => {
            environment
                .selected
                .with_mutation(|mut s| *s = Some(message));
        }
        Action::FinishEdit => {
            environment.selected.with_mutation(|mut s| *s = None);
        }
        Action::Close => {
            context.send_parent(DelegateMessage::Closed);
        }
    }
    Effect::NONE
}
```

Reducers can be nested to form hierachies and have three message / action types:

- `Message`: This is send to child reducers
- `DelegateMessage`: This is send back to the parent reducer of this reducer
- `Action`: This is the internal message in the reducer / view

You can see more by checking out the  [example](examples/gnarl).

## License

Navicula is licensed under the [MIT License](LICENSE)
