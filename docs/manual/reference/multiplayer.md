# Multiplayer

Connections, sessions, players, shared data and messages for multiplayer games.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Setting up a connection](#setting-up-a-connection)
- [Sessions](#sessions)
- [Players](#players)
- [Shared data](#shared-data)
- [Messages](#messages)

## Setting up a connection

Choosing how the computers of the players are connected.

### mplay_init_ipx

`mplay_init_ipx()`

Sets up an IPX network connection for multiplayer.

### mplay_init_tcpip

`mplay_init_tcpip(addrstring)`

Sets up a TCP/IP connection; the address (with optional :port) is only needed when joining a session.

### mplay_init_modem

`mplay_init_modem(initstr, phonenr)`

Sets up a modem connection using an init string and, when joining, a phone number to dial.

### mplay_init_serial

`mplay_init_serial(portno, baudrate, stopbits, parity, flow)`

Sets up a serial connection from port, baud rate, stop bits, parity and flow control; returns success, and port 0 opens a settings dialog.

### mplay_connect_status

`mplay_connect_status()`

Returns the connection type: 0 none, 1 IPX, 2 TCP/IP, 3 modem, 4 serial.

### mplay_end

`mplay_end()`

Closes the current multiplayer connection.

### mplay_ipaddress

`mplay_ipaddress()`

Returns this machine's IP address as a string; slow, so avoid calling every step.

## Sessions

Creating, finding and joining game sessions.

### mplay_session_mode

`mplay_session_mode(move)`

Chooses whether the session host migrates to another computer when the host leaves (default false).

### mplay_session_create

`mplay_session_create(sesname, playnumb, playername)`

Creates a session with a name, maximum player count (0 for unlimited) and your player name; returns success.

### mplay_session_find

`mplay_session_find()`

Looks for sessions still accepting players and returns how many were found.

### mplay_session_name

`mplay_session_name(number)`

Returns the name of the found session with the given index (0-based); call after mplay_session_find.

### mplay_session_join

`mplay_session_join(number, playername)`

Joins the found session with the given index using your player name; returns success.

### mplay_session_status

`mplay_session_status()`

Returns session state: 0 none, 1 created, 2 joined.

### mplay_session_end

`mplay_session_end()`

Ends the session for this player.

## Players

The players in a session.

### mplay_player_find

`mplay_player_find()`

Searches the current session for players and returns the number found.

### mplay_player_name

`mplay_player_name(number)`

Returns the name of the found player with the given index (0 is yourself); call after mplay_player_find.

### mplay_player_id

`mplay_player_id(number)`

Returns the unique id of the found player with the given index (0 is yourself), used for addressing messages.

## Shared data

Values that all players of a session share.

### mplay_data_write

`mplay_data_write(index, value)`

Stores a real or string value in shared data slot index (0 to 1000000).

### mplay_data_read

`mplay_data_read(index)`

Returns the shared data value at slot index (0 to 1000000); all slots start at 0.

### mplay_data_mode

`mplay_data_mode(guaranteed)`

Sets whether shared data uses guaranteed transmission (default true).

## Messages

Sending messages to other players and receiving them.

### mplay_message_send

`mplay_message_send(player, id, value)`

Sends a non-guaranteed message with an integer id and real or string value to a player (0 for all); strings up to 30000 characters.

### mplay_message_send_guaranteed

`mplay_message_send_guaranteed(player, id, value)`

Sends a guaranteed-delivery message with id and value to a player (0 for all); strings up to 30000 characters.

### mplay_message_receive

`mplay_message_receive(player)`

Takes the next queued message from the given player (0 for any); returns whether one was available.

### mplay_message_id

`mplay_message_id()`

Returns the id of the most recently received message.

### mplay_message_value

`mplay_message_value()`

Returns the value of the most recently received message.

### mplay_message_player

`mplay_message_player()`

Returns the sender of the most recently received message.

### mplay_message_name

`mplay_message_name()`

Returns the sender's name for the most recently received message.

### mplay_message_count

`mplay_message_count(player)`

Returns how many messages are queued from the player (0 counts all).

### mplay_message_clear

`mplay_message_clear(player)`

Discards queued messages from the player (0 discards all).
