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
