# Fractal API

The fractal-api crate is a library that give us an abstraction for the
matrix server so we can use with custom types from rust code.

We're using the `reqwest` crate to make the http requests to the matrix
server.

## backend

Here we've the full library used to communicate to the server. This library
works as a server in a thread that waits for orders and emits some responses.

To communicate with the user we'll have a channel that receives `BKCommand`
and sends `BKResponse`. For each command received this lib creates a new
thread that does the work and goes back to wait for the next order.

 * **directory.rs**: Matrix room search API
 * **media.rs**: Thumbnail and media download
 * **register.rs**: Account management, login and logout. The register
   isn't complete because it requires a captcha resolution and that's not
   easy to implement
 * **room.rs**: Room participation API calls:
   * Room state query
   * Send message
   * Message context (used for the infinite scroll)
   * Join/Leave room
   * Mark as read
   * Update room state
   * File attachment
   * Room creation
   * Invitate some user to a room
   * Favourite management
 * **sync.rs**: Initial sync and long polling for server messages to the
   client
 * **types.rs**: Here we define all BKCommands and BKResponses and all the
   Backend structs where we store the backend state that's the current
   user and token after the login and some ids for sync and pagination.
 * **user.rs**: User and member API:
   * Query user information
   * User search

## model

Custom data model structures. Here we've a list of rust struct that
represent the matrix.org types as rust types like:

 * Event
 * Member
 * Message
 * Room

## util

In this module we've a list of useful functions to use in the different
parts of the lib. The main goal for this module is to have a place to
define functions that should be used in several modules.

 * **Macros**: We've a list of macros to simplify http queries syntax and
   to download media files
 * **Matrix events**: There's a lot of functions to parse the json and
   convert to our data model structs
 * **json\_q**: A http-json request abstraction that is used to query the
   server
 * **Identicon**: Functions to generate avatar with a color and letters for
   users and rooms that doesn't have an avatar.
 * **Matrix server calls**: Functions that queries matrix server to get
   information used in different backend modules

## Other modules

There're other small modules that provides some functionality to the
lib:

 * **cache**: Small structure to cache data for a time: `CacheMap`. It's
   like a `HashMap`, but the values are only valid util the timeout
 * **globals**: Constants
 * **error**: Custom error types
