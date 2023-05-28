<div align="center">
  <h1>⚔️  aureus</h1>

  A fantasy MUD shaped by the cultures, mysteries, and magic of the ancient world.
</div>

## Connected -> Playing

**When a user first connects:**

- Spawn a new entity with a `Client` and `Authenticating` component.

**When a user sends a message:**

- If `Authenticating`, all messages are handled by the `auth` systems. When succesfully authenticated,
this component is removed and a `Character` component is added.
- If a `Character`, all messages first go through `parse_command` to be turned into their respective `Command` variant
and sent to `EventWriter<ParsedCommand>`. This system belongs to the `Input` system set that runs _before_ bevys `CoreSet::Update`.
- On every game tick, command systems will iterate through `EventReader<ParsedCommand>` and act on any
event that has their `Command` variant.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/its-danny/aureus/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/its-danny/aureus/blob/main/LICENSE-MIT) license at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed
as above, without any additional terms or conditions.

