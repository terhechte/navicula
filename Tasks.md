# Tasks

- [ ] "render" action in reducers. so we don't need the partialeq / eq
- [ ] Can I return optional effect? that would make the code more readable. At the expensive of having to wrap the actual effect into `Some` (or using into?)
- [ ] can I define a good reducer + view trait so that all the types are accounted for?
- [ ] coalesce to a better Receiver type / EventReceiver. I have too many now.
- [ ] Wrap a flume channel into something so that I can map it into a different type
- [ ] make the reduce function async?

- [ ] handle Environment::AppEvent
- [ ] drop
- [ ] unregister timers and subscriptions
- [ ] have a generic subscription type that can be used with all kinds of types
- [ ] the IntoMessageSender is broken
- [ ] innerEffect
