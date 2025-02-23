# Boxman

A multiplayer game about a boxman.

## Development

### Hosting the server

```bash
cargo run --bin boxman_game -- --server
```

### Running the client

```bash
cargo run --bin boxman_game
```

## Codebase
The codebase is split into three crates:

- `boxman_game`: The main game code.
- `boxman_server`: The server code.
- `boxman_shared`: The shared code.

Right now, the server is a listen server, so technically doesn't need to be a separate crate. But I'm keeping it separate to make it easier to add a dedicated server in the future. Its also just a nice split of concerns.

### Movement Code
- Movement is ran on a fixed timestep. See `boxman_shared/moveable_sim.rs` for the movement simulation.
- In `boxman_game`, you will see `moveable_vis.rs`, this runs on a variable timestep, and interpolates the visual position of the moveable.
- The reconciliation of the visual position is done in `boxman_game/src/net/snapshot.rs`.
- The camera is attached to the visual position. See `player.rs` for more details.
- Input is captured in both a fixed and variable timestep.
    - The looking is done in variable timestep, and consumed in variable timestep.
    - The movement is done in fixed timestep, and consumed in fixed timestep.
        - I might do it in variable timestep in the future, then consume in fixed timestep.
