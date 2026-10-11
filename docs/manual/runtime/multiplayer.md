# Multiplayer

The `mplay_*` functions let several copies of a game, on different computers, play
together over a network. One player **creates a session**; the others **find** and
**join** it. Players then exchange information through **messages** and **shared data**.

Instance runs multiplayer over TCP/IP with its own network protocol. Instance games
talk to Instance games; they can't join sessions of games run by other programs.
Multiplayer works in the desktop versions (Windows, macOS, Linux), not in the browser.

## 1. Connecting

Every multiplayer game starts by initialising the connection, once:

```gml
if (!mplay_init_tcpip("")) {
    show_message("No network available.");
}
```

[`mplay_init_tcpip(addr)`](../reference/multiplayer.md#mplay_init_tcpip) takes an address:

- an empty string looks for sessions on the **local network** (by broadcast);
- a host name or IP address, optionally with `:port`, looks for sessions on **that
  machine** — the way to play over the internet or across networks.

The other connection types (`mplay_init_ipx`, `mplay_init_modem`, `mplay_init_serial`)
are not supported and return false.
[`mplay_connect_status()`](../reference/multiplayer.md#mplay_connect_status) is 2 once TCP/IP
is set up (0 otherwise), and [`mplay_ipaddress()`](../reference/multiplayer.md#mplay_ipaddress)
returns this computer's IP address, so the host can tell it to friends.
[`mplay_end()`](../reference/multiplayer.md#mplay_end) closes everything.

Instance uses port 40810 (TCP and UDP). The firewall must let the game through for
others to join; a host whose port is taken uses one of the next few ports.

## 2. Sessions

```gml
// the host
mplay_session_create("Deathmatch", 4, global.player_name);
```

```gml
// a player looking for a game
var n, i;
n = mplay_session_find();
for (i = 0; i < n; i += 1) {
    show_debug_message("found: " + mplay_session_name(i));
}
if (n > 0) mplay_session_join(0, global.player_name);
```

| Function | Does |
|---|---|
| [`mplay_session_create(name, maxplayers, playername)`](../reference/multiplayer.md#mplay_session_create) | starts a session (0 players = no limit) with you as host |
| [`mplay_session_find()`](../reference/multiplayer.md#mplay_session_find) | searches for sessions of this game that still have room, and returns how many it found. It waits up to about a second (three seconds when an address was given). |
| [`mplay_session_name(n)`](../reference/multiplayer.md#mplay_session_name) | the name of found session `n` (counting from 0) |
| [`mplay_session_join(n, playername)`](../reference/multiplayer.md#mplay_session_join) | joins found session `n`; returns whether it worked |
| [`mplay_session_status()`](../reference/multiplayer.md#mplay_session_status) | 0 not in a session, 1 host, 2 joined |
| [`mplay_session_end()`](../reference/multiplayer.md#mplay_session_end) | leaves the session |
| [`mplay_session_mode(move)`](../reference/multiplayer.md#mplay_session_mode) | call before creating: with `true`, another player takes over as host when the host leaves |

Only sessions of the same game — the same *game id* in Global Game Settings — are
found, so two different games on one network never mix up their sessions. When the
host leaves a session without host migration, the session ends for everyone.

## 3. Players

[`mplay_player_find()`](../reference/multiplayer.md#mplay_player_find) takes a snapshot of
the players in the session and returns how many there are.
[`mplay_player_name(n)`](../reference/multiplayer.md#mplay_player_name) and
[`mplay_player_id(n)`](../reference/multiplayer.md#mplay_player_id) then describe player `n`;
player 0 is always yourself. Ids are positive numbers, unique within the session.

## 4. Messages

Messages are the main way to tell other players what happened:

```gml
// tell everyone where we are (player 0 = all others)
mplay_message_send(0, 1, string(x) + "," + string(y));
```

```gml
// Step event: handle everything that arrived
while (mplay_message_receive(0)) {
    switch (mplay_message_id()) {
        case 1: /* position update */ break;
        case 2: /* a shot */ break;
    }
}
```

- [`mplay_message_send(player, id, value)`](../reference/multiplayer.md#mplay_message_send)
  sends a message with a number `id` (your own message type) and a real or string
  `value`. `player` is a player id, a player name, or 0 for everyone else. Ordinary
  messages may occasionally be lost;
  [`mplay_message_send_guaranteed()`](../reference/multiplayer.md#mplay_message_send_guaranteed)
  makes sure they arrive, in order.
- [`mplay_message_receive(player)`](../reference/multiplayer.md#mplay_message_receive) takes
  the oldest waiting message (from that player, or from anyone with 0) and returns
  whether there was one. Its details are then in
  [`mplay_message_id()`](../reference/multiplayer.md#mplay_message_id),
  [`mplay_message_value()`](../reference/multiplayer.md#mplay_message_value),
  [`mplay_message_player()`](../reference/multiplayer.md#mplay_message_player) and
  [`mplay_message_name()`](../reference/multiplayer.md#mplay_message_name).
- [`mplay_message_count(player)`](../reference/multiplayer.md#mplay_message_count) and
  [`mplay_message_clear(player)`](../reference/multiplayer.md#mplay_message_clear) inspect
  and empty the queue.
- String values are limited to 30000 characters.

## 5. Shared data

Shared data is a big array (indices 0 to 1000000) that is kept the same on every
player's computer:

```gml
mplay_data_write(10, score);           // everyone sees our score in slot 10
other_score = mplay_data_read(11);     // the other player's score
```

[`mplay_data_write(ind, value)`](../reference/multiplayer.md#mplay_data_write) changes a
slot locally and sends the change to everyone;
[`mplay_data_read(ind)`](../reference/multiplayer.md#mplay_data_read) reads the local copy.
All slots start at 0. If two players write the same slot at the same time, everyone
ends up with the same value (the one the host received last). By default changes are
guaranteed to arrive;
[`mplay_data_mode(false)`](../reference/multiplayer.md#mplay_data_mode) makes them faster
but allows losses. Shared data is cleared when you create, join or leave a session; a
joining player receives the current contents.

Shared data is simplest for small amounts of state that change rarely (who is ready, the
chosen level). For fast-changing things like positions, messages are usually better.

## Tips

- Incoming messages and data changes are processed between steps, so read your messages
  every step.
- Test with two copies of the game on one computer: start one as host and join with the
  other using the address `"127.0.0.1"`.
- Keep messages small and send them only when something changes.

See also: [Multiplayer functions](../reference/multiplayer.md) · [The game loop](game-loop.md)
