# Paths

A **path** is a route made of points. An instance can be told to follow a path: a
guard walking a patrol, a ship flying a curve, a platform moving back and forth.
Paths are drawn in the path editor and then started with the **Set Path** action or
the [`path_start`](../reference/game-play.md#path_start) function.

Create one with **Resources > Create Path** and double-click it in the tree to open
the **Path Properties** form.

## The path form

The form has a toolbar at the top, the point list and settings on the left, and the
drawing area on the right.

### Points

Each point has a position (**X**, **Y**) and a speed factor (**sp**). The speed factor
is a percentage of the speed the path was started with: 100 is normal speed, 50 is
half speed, 200 double. The instance changes speed gradually between points.

The list shows all points. Select one to edit its values in the fields below the list.

| Button | What it does |
|---|---|
| Add | Adds a copy of the selected point at the end of the path. |
| Insert | Inserts a copy of the selected point before it. |
| Delete | Deletes the selected point. |

### Drawing with the mouse

| Mouse | Effect |
|---|---|
| Click in an empty spot | Adds a point at the end of the path |
| Drag a point | Moves it |
| Shift+click on a point | Inserts a new point there |
| Right-click on a point | Deletes it |
| Hold Alt | Don't snap to the grid |
| Middle-drag | Moves the view |
| Mouse wheel | Zooms |

The first point is marked so you can see where the path starts.

### Settings

| Setting | What it does |
|---|---|
| Connection kind: Straight lines | The instance moves in straight lines from point to point. |
| Connection kind: Smooth curve | The points shape a smooth curve, which the instance follows instead of the corners of straight lines. |
| Closed | The path continues from the last point back to the first. |
| Precision | How finely a smooth curve is computed (1–8). Higher is smoother. |

### Toolbar

| Command | What it does |
|---|---|
| Undo | Undoes the last change. |
| Clear | Removes all points. |
| Reverse | Reverses the order of the points. |
| Shift | Moves all points by an amount. |
| Mirror | Mirrors the path horizontally. |
| Flip | Flips the path vertically. |
| Rotate | Rotates the path by an angle. |
| Scale | Scales the path by a horizontal and vertical percentage. |
| Center view | Scrolls the view to the middle of the path. |
| Snap X, Y | The grid that new and dragged points snap to. |
| Grid | Shows or hides the grid. |
| Room | Shows a room behind the path, so you can draw it to fit a level. The room is only a backdrop; it isn't changed. |

Mirror, flip, rotate and scale work around the centre of the path.

## Following a path

Paths have no position of their own. When an instance starts a path, the path can be
followed **absolute** (exactly at the coordinates you drew) or **relative** (starting
from the instance's current position). The **Set Path** action and `path_start` also
set the speed and what happens at the end: stop, start again from the beginning,
continue from the current position, or reverse.

When the instance reaches the end, it gets an **End of Path** event (Other events).

```gml
// Follow pth_patrol at speed 2, loop back to the start, relative to here.
path_start(pth_patrol, 2, 1, false);
```

See also: [Movement and collisions](../runtime/movement-and-collisions.md) · [Actions: Paths](actions.md#move) · [Events](events.md#other)
