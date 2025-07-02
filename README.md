# CrabChat Server

## Description

Crab Chat Server is a real-time chat application built using Rust and WebSocket technology. It allows users to create and join chat rooms, send messages, and interact with others in real time.

## Features

*   **Real-time Chat:** Utilizes WebSockets for instant message delivery.
*   **User Authentication:**  Users can register with a name.
*   **Room Management:** Users can create, delete, access, and leave chat rooms.
*   **Public and Private Rooms:**  Rooms can be either public or private.

## How to run
- `-p` or `--port` to pass the port

```sh
crabchatserver -p <port>
```

## Technologies Used

*   **Rust:** The primary programming language.
*   **Tokio:** An asynchronous runtime for Rust.
*   **Tokio Tungstenite:**  A WebSocket library for Tokio.
*   **Serde:** For serialization and deserialization of data.
*   **UUID:** For generating unique user identifiers.
*   **Tracing:** For logging and diagnostics.

## Getting Started

### Prerequisites

*   [Rust toolchain](https://rustup.rs/)
*   [Tokio](https://tokio.rs/)
*   [Tungstenite](https://github.com/snapview/tungstenite-rs)


## Project Structure

*   `src/`: Contains the source code.
    *   `auth/`: Authentication-related logic.
    *   `infra/`: Infrastructure code (enums, models).
    *   `message/`: Message processing logic.
    *   `room/`: Room management logic.
    *   `types/`: Type definitions.
    *   `websocket/`: WebSocket handling.
    *   `main.rs`: Entry point of the application.


## Contributing

Contributions are welcome!  Please feel free to submit pull requests, report issues, or suggest new features.
