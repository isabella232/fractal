# Fractal GTK

## app

The Gtk Application. In this module we link each Gtk event with a function
or a closure. The app has a reference to the `AppOp` so from each signal we
can call to the methods of the `AppOp`.

From here we launch the Gtk application and we've two main loops to
communicate with the matrix server and to make internal calls.

 * **backend\_loop**: This loop listen for api events and calls the
   corresponding `AppOp` method for each `BKResponse`.
 * **app\_loop**: This loop is used to send task to the `AppOp` from some
   closure that's inside the `AppOp`. We've a channel we're we can send a
   command to execute in the gtk main context.

 * **connect.rs**: Inside this module we make the signal connection for
   each widget in the interface: res/ui/\*.xml

## appop

The `AppOp` stores the application state, all the information is stored in
this struct and all methods modify the interface or sent events to the API.

 * **about.rs**: The about dialog
 * **attach.rs**: Attach image when copy-pasting in the message box
 * **directory.rs**: Matrix directory search methods
 * **invite.rs**: Invite user dialog, to invite users to a room and dialogs
   to accept or reject invitations
 * **login.rs**: Login and logout management
 * **member.rs**: Member list management and search
 * **message.rs**: Message history functions, like:
   * Send new message
   * Attach a file
   * Infinite scroll
   * Mark as read
 * **mod.rs**: Here we've the struct and the initialization methods
 * **notifications.rs**: Room notifications methods
 * **notify.rs**: Inapp notifications and system notifications
 * **room.rs**: Room methods, in this module we've a lot of functions for
   the interaction with rooms:
   * New/Update/Delete rooms
   * Room change when clicking on the Room in the sidebar
 * **start\_chat.rs**: Direct chat dialog
 * **state.rs**: Application state structs (Login|Chat|Directory|Loading)
 * **sync.rs**: Sync loop methods
 * **user.rs**: User info and management

## passwd

Password and token storage module. In this module there're the code to
store and retrieve the password and the token to interact with the matrix
server.

By default the secret service is used here.

## widgets

The widgets mod provide a list of **custom** widgets to render in the
application.

### autocomplete.rs

Widget used for the username autocompletion with the tab, uses the room
members to show a list of members that match the text written and manages
the click and keyboard events.

### avatar.rs

Circle avatar widget to show user and room photos, this widget will show a
default icon when loading and after the loading will show the real image.
Can be a circle or a square.

### divider.rs

Widget to show the last readed message, a simple blue line with a text in
the middle.

### member.rs

This widget shows a room member, the avatar and the username. This widget
uses the API to query the username and the avatar.

### message.rs

Widget used to show a message in the room message list. We're grouping same
user consecutive messages using two different renders for the same widget,
the normal, with the username, date and the avatar and the small one that
shows only the text.

### roomlist.rs

The left sidebar show the roomlist. This `RoomList` has three `RGroup`, one
for invites, another one for favourites and the final one for the rest of
rooms. This widget manages the drag & drop between groups.

Each `RGroup` has a list of rooms defined in the `roomrow` mod.

 * **roomrow.rs**: A row in the room list, this shows the room avatar, the
   room name and the number of notifications.

### room.rs

This widget is used to show each room in the room directory, when the user
search for a room. Here we show the avatar, the name and the topic and a
button to join to that room.

## Other modules

There're other small modules that provides some functionality to the
application:

 * **cache**: Room list and messages storage in a json file
 * **globals**: Interface constants
 * **util**: Utils functions and macros
 * **static\_resources**: Loading of static resources from gresource
 * **uibuilder**: Construct a gtk::Builder object with all .ui files
