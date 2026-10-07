# Multiplayer (`mplay_*`) — behavioural specification and gm81-clean wire protocol

Source: manual chapter 413 (`413_00` … `413_05`), paraphrased. Part A describes the
GML-visible behaviour. Part B is **gm81-clean's own network protocol** — it is *not*
GameMaker's (GM 8.1 used DirectPlay) and gm81-clean games cannot talk to games run
by the original runner. Open items: `OPEN_QUESTIONS.md` section `[MP]`.

---

# Part A — GML behaviour

## A.1 Concepts `[413_00]`
* **Connection**: the transport a game uses (IPX, TCP/IP, modem, serial in GM).
  Initialised exactly once per game run `[413_01]`.
* **Session**: one running multiplayer game on a connection; created by one game
  instance (the *host*), joined by others `[413_02]`. Sessions are tagged with the
  game's identity so different games (and different versions — the "game id" of the
  Loading settings, `file-formats.md` `game_id`) never see each other's sessions
  `[413_02]`.
* **Player**: each game instance in a session `[413_03]`; has a name and a unique
  numeric id.
* Two communication mechanisms: **shared data** (a global array kept equal on all
  machines) and **messages** (queued, per sender) `[413_04]`, `[413_05]`.

## A.2 Connection functions `[413_01]`

| Function | gm81-clean behaviour |
|---|---|
| `mplay_init_ipx()` | Not supported: returns false, status unchanged. |
| `mplay_init_tcpip(addr)` | Initialise TCP/IP. `addr` = host name or IPv4/IPv6 address, optionally followed by `:port`; may be empty. Needed only for joining a session on a specific machine; empty = LAN mode (discovery by broadcast, Part B). Returns true if a socket layer is available (always on desktop). Stores `addr` for later `session_find`. |
| `mplay_init_modem(initstr,phonenr)` | Not supported: returns false. |
| `mplay_init_serial(portno,baudrate,stopbits,parity,flow)` | Not supported: returns false (GM's argument meanings: port 1–4 or 0 = settings dialog; stop bits 0=1, 1=1.5, 2=2; parity 0 none,1 odd,2 even,3 mark; flow 0 none,1 xon/xoff,2 rts,3 dtr,4 rts+dtr). |
| `mplay_connect_status()` | 0 none, 1 IPX, 2 TCP/IP, 3 modem, 4 serial. gm81-clean: 0 or 2. |
| `mplay_end()` | Ends the session (as `mplay_session_end`) and the connection; status → 0. |
| `mplay_ipaddress()` | This machine's IP address as a string (e.g. `"192.168.1.5"`). The manual warns it is slow. **Recommended:** the first non-loopback IPv4 address of an up interface (the one with a default route if determinable); `"127.0.0.1"` if none. Cache for ~5 s. |

A second `mplay_init_*` call while connected: **recommended** end the current
connection first, then initialise anew (the manual says to call it exactly once).

## A.3 Sessions `[413_02]`

| Function | Semantics |
|---|---|
| `mplay_session_mode(move)` | Whether the host role migrates to another player when the host leaves (default false). Must be called before creating. |
| `mplay_session_create(sesname,playnumb,playername)` | Create a session named `sesname` allowing at most `playnumb` players (0 = unlimited), with yourself as first player `playername`. Returns success. Fails if not connected or already in a session. |
| `mplay_session_find()` | Search for sessions of this game that still accept players; returns the count. Blocking (Part B §B.3). Replaces the previously found list. |
| `mplay_session_name(numb)` | Name of found session `numb` (0-based). Valid only after `find`; out of range → `""`. |
| `mplay_session_join(numb,playername)` | Join found session `numb` as `playername`. Returns success. Blocking. |
| `mplay_session_status()` | 0 no session, 1 created (you are host), 2 joined. |
| `mplay_session_end()` | Leave the session (host: end or hand over the session, §A.7). |

## A.4 Players `[413_03]`

| Function | Semantics |
|---|---|
| `mplay_player_find()` | Snapshot the current player list, return its length. |
| `mplay_player_name(numb)` | Name of player `numb` in the snapshot; **0 is always yourself**. Out of range → `""`. |
| `mplay_player_id(numb)` | Unique id of player `numb` (0 = yourself). Out of range → 0. |

Snapshot order: yourself, then the other players in the order they joined
(host first if you are not the host). Ids are positive integers, unique within the
session; never 0 (0 means "all"/"any" in message functions).

## A.5 Shared data `[413_04]`

* 1 000 001 slots, indices 0..1000000, each a real or string, initially all 0.
* `mplay_data_write(ind,val)`: set slot locally at once and propagate to all players.
* `mplay_data_read(ind)`: local value of the slot.
* `mplay_data_mode(guar)`: true (default) = guaranteed propagation; false = fast,
  may be lost.
* All players converge on the same value per slot (last write in host order wins,
  Part B §B.6).
* **Recommended:** index is rounded to integer; out of range → write ignored, read
  returns 0. Store sparsely (hash map). Shared data is cleared when a session is
  created or joined (joiners receive the host's current table) and on
  `mplay_session_end`.
* Writes while not in a session: stored locally only.

## A.6 Messages `[413_05]`

| Function | Semantics |
|---|---|
| `mplay_message_send(player,id,val)` | Send message (`id` integer, `val` real or string) to `player` (an id, a name string, or 0 = all other players). Non-guaranteed. |
| `mplay_message_send_guaranteed(player,id,val)` | Same, guaranteed delivery. |
| `mplay_message_receive(player)` | Take the oldest queued message from `player` (id, name, or 0 = any). Returns whether one was taken; if so it becomes the "last received message". |
| `mplay_message_id()` | Id of the last received message. |
| `mplay_message_value()` | Value of the last received message. |
| `mplay_message_player()` | Sender id of the last received message. |
| `mplay_message_name()` | Sender name of the last received message. |
| `mplay_message_count(player)` | Number of queued messages from `player` (0 = all). |
| `mplay_message_clear(player)` | Discard queued messages from `player` (0 = all). |

Rules:
* String values longer than 30000 characters are not allowed `[413_05]`;
  **recommended** truncate to 30000.
* Player given by name: the first player in the current player list with that
  name (exact, case-sensitive — **recommended**) `[413_05]`. Unknown name/id: the
  send is silently dropped (returns nothing); receive/count treat it as "no
  messages".
* A message to all (0) is delivered to every player except the sender.
* Sending to yourself (own id): **recommended** delivered to your own queue.
* Before any receive, `mplay_message_id/value/player/name` return 0, 0, 0, `""`.
* `id` is sent as a real and returned as sent (**recommended**; the manual calls it
  an integer).
* Messages from one sender arrive in send order when guaranteed; non-guaranteed
  messages may be lost or reordered relative to others.
* Message queues survive until read/cleared; they are cleared on session end.
* Functions return 0/""/false when not in a session.

## A.7 Session end and host migration
* A non-host calling `mplay_session_end`: it leaves; others see the player vanish
  from the next `mplay_player_find`. Its status → 0.
* The host calling `mplay_session_end` (or the host's game closing):
  * mode `move = false` (default): the session ends for everyone; every remaining
    player's `mplay_session_status()` becomes 0 (**recommended**; DirectPlay
    behaviour not documented, MP-2).
  * mode `move = true`: another player becomes host; the session continues for the
    rest (Part B §B.8).
* Lost connection to the host is treated like the host ending.

## A.8 Threading model (implementation)
Networking runs on a background thread (or async task). The game thread only
touches thread-safe queues/tables: incoming messages and shared-data updates are
applied **between steps** (recommended: at the start of each step, before Begin
Step events) — except that `mplay_data_read`/`mplay_message_receive` may also drain
the inbound queue immediately so that busy-waiting GML loops make progress.

---

# Part B — gm81-clean TCP/IP protocol (version 1)

**This is gm81-clean's own protocol, designed for this project. It is not
compatible with GameMaker/DirectPlay.**

## B.1 Topology and transports
* Star topology: the **host** (session creator) is the server; every other player
  keeps one TCP connection to the host. The host relays all traffic. Players never
  connect to each other directly.
* TCP carries all control traffic, guaranteed messages and guaranteed shared-data
  writes.
* Optional UDP datagram channel (capability `UDP_DATA`, §B.4) for non-guaranteed
  traffic. An implementation may omit it and send non-guaranteed traffic over TCP
  (this is allowed: non-guaranteed only means "may be lost").
* LAN discovery: UDP broadcast.
* Default port **40810** for both TCP (sessions) and UDP (discovery and data).
  `mplay_init_tcpip("host:port")` overrides the TCP port to contact; a host whose
  default TCP port is busy tries 40811..40819 and advertises the actual port in
  discovery replies.
* IPv4 required; IPv6 supported for addressed joins when the address is IPv6.

## B.2 Encoding
All integers little-endian. Types:
* `u8, u16, u32, i32, f64` — fixed size.
* `str` = `u32 byte_length` + bytes (the runtime's string bytes; no terminator).
  Max 65 536 bytes for names, 30 000 characters' encoding for values.
* `value` = `u8 tag` + payload: tag 0 = real (`f64`), tag 1 = string (`str`).

### TCP framing
```
u32 length            (bytes after this field; 1 ≤ length ≤ 1 048 576)
u8  type
u8[length-1] payload
```
A frame exceeding the limit or with an unknown type from a peer that negotiated
the same protocol version is a protocol error: close the connection.

### UDP datagram format
```
u8[4] magic  = "GMMP"
u8    version = 1
u8    type
u8[]  payload              (whole datagram ≤ 1200 bytes)
```

## B.3 Discovery (`mplay_session_find`)
* **LAN mode** (empty address): the finder sends `DISCOVER` (UDP type 0x01) to
  `255.255.255.255:40810` and to every IPv4 interface's directed broadcast address,
  then collects `SESSION_INFO` replies for **1000 ms** (recommended), re-sending
  the query once at 300 ms. Payload of `DISCOVER`:
  `u32 nonce; i32 game_id; u16 proto_version`.
* **Addressed mode** (address given): the finder sends `DISCOVER` by unicast UDP to
  `addr:port` **and** opens a TCP connection to `addr:port` sending frame
  `LIST_SESSIONS` (0x20, payload `i32 game_id; u16 proto_version`), to which the
  host answers `SESSION_LIST` (0x21: `u16 count; count × session_info_body`) and
  closes. Either answer suffices; wait at most 3000 ms.
* Hosts listen on UDP 40810 (bound with address reuse so several hosts on one
  machine all receive broadcasts) and answer only queries whose `game_id` and
  `proto_version` match, with `SESSION_INFO` (UDP type 0x02):
  ```
  u32 nonce (echoed)
  session_info_body:
    u64 session_token      (random, identifies the session instance)
    u16 tcp_port
    str session_name
    str host_player_name
    u16 player_count
    u16 max_players        (0 = unlimited)
    u8  accepting          (1 if not full and not closing)
  ```
* `mplay_session_find` lists only sessions with `accepting = 1`, de-duplicated by
  `session_token`, in order of first reply. The reply's source IP + `tcp_port` is
  remembered for joining.

## B.4 Joining (TCP)
Client connects to the host's TCP port and sends `HELLO` (0x10):
```
u8[6] magic "GM81MP"; u16 proto_version (=1); i32 game_id;
u64 session_token (0 when joining by address without a prior find);
u32 capabilities (bit0 UDP_DATA); str player_name
```
Host answers `WELCOME` (0x11) or `REJECT` (0x12) and, after REJECT, closes:
```
WELCOME: u32 your_player_id; u32 host_player_id; u64 session_token;
         str session_name; u16 max_players; u32 capabilities (agreed);
         u32 udp_cookie (0 if no UDP);
         u16 player_count; player_count × { u32 id; str name }   (join order, host first)
         u32 data_count;   data_count × { u32 index; value }      (all non-zero slots)
REJECT:  u8 reason   (1 session full, 2 wrong game id, 3 protocol version,
                      4 session closing, 5 malformed hello)
```
`mplay_session_join` returns true after `WELCOME`, false on `REJECT`, connection
failure, or no answer within **5000 ms**. The host then broadcasts
`PLAYER_JOINED` (0x13: `u32 id; str name`) to every other player.

Player ids: the host assigns them from a per-session counter starting at 1
(host = 1), never reusing an id within the session.

UDP channel (if agreed): the client sends UDP `UDP_BIND` (type 0x03, payload
`u32 udp_cookie; u32 player_id`) to the host's UDP port every 500 ms until it
receives UDP `UDP_BIND_ACK` (0x04, same payload); the host then uses the datagram's
source address for that player.

## B.5 Messages
Client → host `MSG_SEND` (TCP 0x30, or UDP type 0x30 when non-guaranteed and UDP is
agreed):
```
u32 to (0 = all); u8 guaranteed (0/1); f64 msg_id; value
```
(UDP form prefixes `u32 udp_cookie; u32 sender_id`.)
Host → recipient(s) `MSG_DELIVER` (0x31, same transport choice):
```
u32 from; f64 msg_id; value
```
* The host fills in `from` from the connection it arrived on (no spoofing).
* `to = 0`: deliver to every player except the sender. `to = host id`: host queues
  it locally. Unknown `to`: dropped.
* Names are resolved to ids by the sender before sending (§A.6).
* The host's own sends go directly to recipients as `MSG_DELIVER`.
* The receiving runtime appends `{from, from_name (looked up at arrival), id,
  value}` to its message queue.

## B.6 Shared data
* Writer applies the value locally, then sends `DATA_SET` (TCP 0x40 when
  `mplay_data_mode(true)`, otherwise UDP 0x40 if agreed, else TCP):
  `u32 index; value`.
* The host applies it to its table **in arrival order** and sends `DATA_UPDATE`
  (0x41: `u32 index; value`) to **all** players, *including the writer*. Players
  apply every `DATA_UPDATE`. Because every player finally applies the host's order,
  all tables converge.
* Host writes: applied locally, `DATA_UPDATE` to all clients.
* Non-guaranteed updates may be lost; the host additionally re-broadcasts
  (recommended) any slot changed via UDP over TCP after 1 s if no newer write
  arrived, so tables still converge eventually.

## B.7 Keep-alive and leaving
* `PING` (0x50, `u32 seq`) / `PONG` (0x51, `u32 seq`) every 2 s in both directions
  when idle; a peer silent for **10 s** is considered gone.
* Client leaves: sends `BYE` (0x52, empty) and closes. The host removes it and
  broadcasts `PLAYER_LEFT` (0x14: `u32 id`). A dropped connection is the same.
* Host ends without migration: sends `SESSION_CLOSED` (0x53, `u8 reason`: 0 ended,
  1 shutting down) to all and closes; clients' session status becomes 0.

## B.8 Host migration (`mplay_session_mode(true)`)
* While migration is enabled the host also sends `PEER_ADDR` (0x15:
  `u32 id; str ip; u16 tcp_port_hint`) for every player to every player, where the
  IP is as seen by the host and `tcp_port_hint` is the port that player would listen
  on (default 40810).
* When the host ends the session: it sends `MIGRATE` (0x54: `u32 new_host_id`),
  choosing the remaining player with the **lowest id**, then closes.
* The new host starts listening (TCP/UDP as in §B.1) and keeps the session token,
  name, player ids, id counter (next id = max + 1) and shared-data table. Other
  players connect to it and send `RESUME` (0x16: `u64 session_token; u32 player_id`)
  instead of `HELLO`; the new host answers `WELCOME` (with the same ids).
* Players that cannot reach the new host within 5 s are dropped
  (`PLAYER_LEFT`). The new host's `mplay_session_status()` becomes 1; the others
  stay 2.
* If the host disappears without `MIGRATE` (crash/timeout), the remaining players
  run the same election locally (lowest id becomes host) and proceed as above.

## B.9 Message type registry

| Type | Name | Transport | Direction |
|---|---|---|---|
| 0x01 | DISCOVER | UDP | finder → hosts |
| 0x02 | SESSION_INFO | UDP | host → finder |
| 0x03 | UDP_BIND | UDP | client → host |
| 0x04 | UDP_BIND_ACK | UDP | host → client |
| 0x10 | HELLO | TCP | client → host |
| 0x11 | WELCOME | TCP | host → client |
| 0x12 | REJECT | TCP | host → client |
| 0x13 | PLAYER_JOINED | TCP | host → clients |
| 0x14 | PLAYER_LEFT | TCP | host → clients |
| 0x15 | PEER_ADDR | TCP | host → clients |
| 0x16 | RESUME | TCP | client → new host |
| 0x20 | LIST_SESSIONS | TCP | finder → host |
| 0x21 | SESSION_LIST | TCP | host → finder |
| 0x30 | MSG_SEND | TCP/UDP | client → host |
| 0x31 | MSG_DELIVER | TCP/UDP | host → client |
| 0x40 | DATA_SET | TCP/UDP | client → host |
| 0x41 | DATA_UPDATE | TCP/UDP | host → clients |
| 0x50 | PING | TCP | both |
| 0x51 | PONG | TCP | both |
| 0x52 | BYE | TCP | client → host |
| 0x53 | SESSION_CLOSED | TCP | host → clients |
| 0x54 | MIGRATE | TCP | host → clients |

Unknown UDP datagrams (bad magic/version/cookie) are ignored silently.

## B.10 Security notes
* Validate all lengths before allocating; cap the player count at 256 even when
  `max_players = 0`; cap shared-data slots at the GML range.
* Never execute received strings; games may (via `execute_string`), which is their
  responsibility.
* No encryption or authentication in v1 (LAN/trusted use, as in GM).

---

## Constants
`fnames` defines no `mplay` constants. Numeric meanings used by the functions
(not GML constants): connection status 0–4, session status 0–2 (§A.2, §A.3).
