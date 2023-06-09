<div align="center">
  <h1>⚔️  aureus</h1>

  A fantasy MUD shaped by the cultures, mysteries, and magic of the ancient world.
</div>

## Dev

**Requirements:**

- [docker-compose](https://docs.docker.com/compose/)
- [rust](https://rustup.rs/)
- [sqlx-cli](https://lib.rs/crates/sqlx-cli)
- A MUD client, e.g. [blightmud](https://github.com/blightmud/blightmud)

```bash
cp .env.example .env
docker-compose up    # Start PostgreSQL
sqlx create database # Create dev database
cargo run -p server  # Starts the server at 127.0.0.1:3000
```

**Resources:**

- https://docs.rs/bevy/latest/bevy/
- https://bevy-cheatbook.github.io/
- https://docs.rs/sqlx/latest/sqlx/

### How it works

**When a user first connects:**

- Spawn a new entity with a [`Client`](https://github.com/its-danny/aureus/blob/main/server/src/player/components.rs)
and [`Authenticating`](https://github.com/its-danny/aureus/blob/main/server/src/auth/components.rs) component.

**When a user sends a message:**

- If authenticating, all messages are handled by the [`auth systems`](https://github.com/its-danny/aureus/blob/main/server/src/auth/systems.rs).
When succesfully authenticated, this component is removed and an
[`Online`](https://github.com/its-danny/aureus/blob/main/server/src/player/components.rs) component is added.
- When not authenticating, all messages first go through [`parse_command`](https://github.com/its-danny/aureus/blob/main/server/src/input/systems.rs)
to be turned into their respective [`Command`](https://github.com/its-danny/aureus/blob/main/server/src/input/events.rs)
variant and sent to `EventWriter<ParsedCommand>`. This system belongs to the `Input` system set that runs _before_ bevys `CoreSet::Update`.
- On every game tick, command systems will iterate through `EventReader<ParsedCommand>` and act on their respective events.

**Positions**

A `Tile` is the only entity with a `Position` and its position is relative to the `Zone` it belongs to. All other positioned entites
are children of a tile.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/its-danny/aureus/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/its-danny/aureus/blob/main/LICENSE-MIT) license at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed
as above, without any additional terms or conditions.

