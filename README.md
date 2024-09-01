<div align="center">
  <img src="./meta/logo.png">

  A fantasy MUD shaped by the cultures, mysteries, and magic of the ancient world.

  ![Not Maintained](https://img.shields.io/maintenance/no/2024)
</div>

## Dev

**Requirements:**

- [docker-compose](https://docs.docker.com/compose/)
- [rust](https://rustup.rs/)
- [just](https://github.com/casey/just)
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

## License

The code is licensed under either of [Apache License, Version 2.0](https://github.com/its-danny/kindara/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/its-danny/kindara/blob/main/LICENSE-MIT) license at your option. The content and assets
are licensed under [CC BY-NC-SA 4.0](https://github.com/its-danny/kindara/blob/main/LICENSE-CC-BY-NC-SA).

## Contributing

Contributions, specifically typo corrections and bug fixes, are welcome. Please note that this is a hobby project,
so new features or content are not sought after. All contributions will fall under the existing project licenses.

