# Fractal Architecture

Fractal is divided in two main crates, `fractal-api` and `fractal-gtk`. The
first one defines an abstraction of the Matrix.org api and the second one
does the gtk application, drawing widgets and interface, storing the app
state and managing user events.

These two crates shares information using a `channel` so the `api` can send
information to the `gtk` and the `gtk` can make request to the `api`.
