# Data structures

Stacks, queues, lists, maps, priority queues and grids.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [General](#general)
- [Stacks](#stacks)
- [Queues](#queues)
- [Lists](#lists)
- [Maps](#maps)
- [Priority queues](#priority-queues)
- [Grids](#grids)

## General

Settings shared by all data structures.

### ds_set_precision

`ds_set_precision(precision)`

Sets the precision used when comparing real values in data structures.

## Stacks

Last in, first out collections.

### ds_stack_create

`ds_stack_create()`

Creates a new stack and returns its integer id for use in other functions.

### ds_stack_destroy

`ds_stack_destroy(id)`

Destroys the stack with the given id, freeing its memory.

### ds_stack_clear

`ds_stack_clear(id)`

Removes all data from the stack without destroying it.

### ds_stack_copy

`ds_stack_copy(id, source)`

Copies the contents of the source stack into the stack with the given id.

### ds_stack_size

`ds_stack_size(id)`

Returns the number of entries stored in the stack.

### ds_stack_empty

`ds_stack_empty(id)`

Returns whether the stack is empty (size 0).

### ds_stack_push

`ds_stack_push(id, value)`

Pushes a value onto the stack.

### ds_stack_pop

`ds_stack_pop(id)`

Returns the top value and removes it from the stack.

### ds_stack_top

`ds_stack_top(id)`

Returns the top value of the stack without removing it.

### ds_stack_write

`ds_stack_write(id)`

Serializes the stack to a string that can be saved, e.g. to a file.

### ds_stack_read

`ds_stack_read(id, str)`

Restores the stack from a string produced by the corresponding write function.

## Queues

First in, first out collections.

### ds_queue_create

`ds_queue_create()`

Creates a new queue and returns its integer id for use in other functions.

### ds_queue_destroy

`ds_queue_destroy(id)`

Destroys the queue with the given id, freeing its memory.

### ds_queue_clear

`ds_queue_clear(id)`

Removes all data from the queue without destroying it.

### ds_queue_copy

`ds_queue_copy(id, source)`

Copies the contents of the source queue into the queue with the given id.

### ds_queue_size

`ds_queue_size(id)`

Returns the number of entries stored in the queue.

### ds_queue_empty

`ds_queue_empty(id)`

Returns whether the queue is empty (size 0).

### ds_queue_enqueue

`ds_queue_enqueue(id, value)`

Adds a value to the queue.

### ds_queue_dequeue

`ds_queue_dequeue(id)`

Returns the oldest value in the queue and removes it.

### ds_queue_head

`ds_queue_head(id)`

Returns the oldest value in the queue without removing it.

### ds_queue_tail

`ds_queue_tail(id)`

Returns the most recently added value in the queue without removing it.

### ds_queue_write

`ds_queue_write(id)`

Serializes the queue to a string that can be saved, e.g. to a file.

### ds_queue_read

`ds_queue_read(id, str)`

Restores the queue from a string produced by the corresponding write function.

## Lists

Ordered collections that can be indexed, sorted and shuffled.

### ds_list_create

`ds_list_create()`

Creates a new list and returns its integer id for use in other functions.

### ds_list_destroy

`ds_list_destroy(id)`

Destroys the list with the given id, freeing its memory.

### ds_list_clear

`ds_list_clear(id)`

Removes all data from the list without destroying it.

### ds_list_copy

`ds_list_copy(id, source)`

Copies the contents of the source list into the list with the given id.

### ds_list_size

`ds_list_size(id)`

Returns the number of entries stored in the list.

### ds_list_empty

`ds_list_empty(id)`

Returns whether the list is empty (size 0).

### ds_list_add

`ds_list_add(id, value)`

Appends a value to the end of the list.

### ds_list_insert

`ds_list_insert(id, pos, value)`

Inserts a value at position pos (0-based) in the list.

### ds_list_replace

`ds_list_replace(id, pos, value)`

Replaces the value at position pos in the list.

### ds_list_delete

`ds_list_delete(id, pos)`

Deletes the value at position pos (0-based) from the list.

### ds_list_find_index

`ds_list_find_index(id, value)`

Returns the position of the value in the list, or -1 if not found.

### ds_list_find_value

`ds_list_find_value(id, pos)`

Returns the value stored at the given list position.

### ds_list_sort

`ds_list_sort(id, ascending)`

Sorts the list ascending if ascend is true, otherwise descending.

### ds_list_shuffle

`ds_list_shuffle(id)`

Randomly reorders the values in the list.

### ds_list_write

`ds_list_write(id)`

Serializes the list to a string that can be saved, e.g. to a file.

### ds_list_read

`ds_list_read(id, str)`

Restores the list from a string produced by the corresponding write function.

## Maps

Collections of key-value pairs.

### ds_map_create

`ds_map_create()`

Creates a new map and returns its integer id for use in other functions.

### ds_map_destroy

`ds_map_destroy(id)`

Destroys the map with the given id, freeing its memory.

### ds_map_clear

`ds_map_clear(id)`

Removes all data from the map without destroying it.

### ds_map_copy

`ds_map_copy(id, source)`

Copies the contents of the source map into the map with the given id.

### ds_map_size

`ds_map_size(id)`

Returns the number of entries stored in the map.

### ds_map_empty

`ds_map_empty(id)`

Returns whether the map is empty (size 0).

### ds_map_add

`ds_map_add(id, key, value)`

Adds a key-value pair to the map.

### ds_map_replace

`ds_map_replace(id, key, value)`

Replaces the value stored for an existing key.

### ds_map_delete

`ds_map_delete(id, key)`

Deletes a key and its value from the map (only one entry if duplicates).

### ds_map_exists

`ds_map_exists(id, key)`

Returns whether the key exists in the map.

### ds_map_find_value

`ds_map_find_value(id, key)`

Returns the value associated with the key.

### ds_map_find_previous

`ds_map_find_previous(id, key)`

Returns the largest key smaller than the given key (the key, not its value).

### ds_map_find_next

`ds_map_find_next(id, key)`

Returns the smallest key larger than the given key.

### ds_map_find_first

`ds_map_find_first(id)`

Returns the smallest key in the map.

### ds_map_find_last

`ds_map_find_last(id)`

Returns the largest key in the map.

### ds_map_write

`ds_map_write(id)`

Serializes the map to a string that can be saved, e.g. to a file.

### ds_map_read

`ds_map_read(id, str)`

Restores the map from a string produced by the corresponding write function.

## Priority queues

Collections that hand out values in order of their priority.

### ds_priority_create

`ds_priority_create()`

Creates a new priority queue and returns its integer id for use in other functions.

### ds_priority_destroy

`ds_priority_destroy(id)`

Destroys the priority queue with the given id, freeing its memory.

### ds_priority_clear

`ds_priority_clear(id)`

Removes all data from the priority queue without destroying it.

### ds_priority_copy

`ds_priority_copy(id, source)`

Copies the contents of the source priority queue into the priority queue with the given id.

### ds_priority_size

`ds_priority_size(id)`

Returns the number of entries stored in the priority queue.

### ds_priority_empty

`ds_priority_empty(id)`

Returns whether the priority queue is empty (size 0).

### ds_priority_add

`ds_priority_add(id, value, priority)`

Adds a value with the given priority to the priority queue.

### ds_priority_change_priority

`ds_priority_change_priority(id, value, priority)`

Changes the priority of a value already in the priority queue.

### ds_priority_find_priority

`ds_priority_find_priority(id, value)`

Returns the priority of the given value.

### ds_priority_delete_value

`ds_priority_delete_value(id, value)`

Removes the given value and its priority from the queue.

### ds_priority_delete_min

`ds_priority_delete_min(id)`

Returns the lowest-priority value and removes it.

### ds_priority_find_min

`ds_priority_find_min(id)`

Returns the lowest-priority value without removing it.

### ds_priority_delete_max

`ds_priority_delete_max(id)`

Returns the highest-priority value and removes it.

### ds_priority_find_max

`ds_priority_find_max(id)`

Returns the highest-priority value without removing it.

### ds_priority_write

`ds_priority_write(id)`

Serializes the priority queue to a string that can be saved, e.g. to a file.

### ds_priority_read

`ds_priority_read(id, str)`

Restores the priority queue from a string produced by the corresponding write function.

## Grids

Two-dimensional arrays with operations on regions.

### ds_grid_create

`ds_grid_create(width, height)`

Creates a grid of the given width and height and returns its id.

### ds_grid_destroy

`ds_grid_destroy(id)`

Destroys the grid with the given id, freeing its memory.

### ds_grid_copy

`ds_grid_copy(id, source)`

Copies the contents of the source grid into the grid with the given id.

### ds_grid_resize

`ds_grid_resize(id, width, height)`

Resizes the grid to a new width and height, keeping existing cell values.

### ds_grid_width

`ds_grid_width(id)`

Returns the width of the grid.

### ds_grid_height

`ds_grid_height(id)`

Returns the height of the grid.

### ds_grid_clear

`ds_grid_clear(id, value)`

Sets every cell of the grid to the given number or string.

### ds_grid_set

`ds_grid_set(id, x, y, value)`

Sets one grid cell to a number or string value.

### ds_grid_add

`ds_grid_add(id, x, y, value)`

Adds a value to a grid cell; strings are concatenated.

### ds_grid_multiply

`ds_grid_multiply(id, x, y, value)`

Multiplies a grid cell by a value; numbers only.

### ds_grid_set_region

`ds_grid_set_region(id, x1, y1, x2, y2, value)`

Sets all cells in a rectangular region to a value.

### ds_grid_add_region

`ds_grid_add_region(id, x1, y1, x2, y2, value)`

Adds a value to all cells in a region; strings are concatenated.

### ds_grid_multiply_region

`ds_grid_multiply_region(id, x1, y1, x2, y2, value)`

Multiplies all cells in a region by a value; numbers only.

### ds_grid_set_disk

`ds_grid_set_disk(id, xcenter, ycenter, r, value)`

Sets all cells within a disk (center xcenter,ycenter, radius r) to a value.

### ds_grid_add_disk

`ds_grid_add_disk(id, xcenter, ycenter, r, value)`

Adds a value to all cells within the given disk.

### ds_grid_multiply_disk

`ds_grid_multiply_disk(id, xcenter, ycenter, r, value)`

Multiplies all cells within the given disk by a value.

### ds_grid_set_grid_region

`ds_grid_set_grid_region(id, source, x1, y1, x2, y2, xpos, ypos)`

Copies a region of a source grid into this grid at (xpos,ypos); also usable to copy within one grid.

### ds_grid_add_grid_region

`ds_grid_add_grid_region(id, source, x1, y1, x2, y2, xpos, ypos)`

Adds a region of a source grid onto this grid at (xpos,ypos); the two grids may be the same.

### ds_grid_multiply_grid_region

`ds_grid_multiply_grid_region(id, source, x1, y1, x2, y2, xpos, ypos)`

Multiplies this grid by a region of a source grid placed at (xpos,ypos); numbers only.

### ds_grid_get

`ds_grid_get(id, x, y)`

Returns the value of the grid cell at (x,y).

### ds_grid_get_sum

`ds_grid_get_sum(id, x1, y1, x2, y2)`

Returns the sum of cells in a region; numeric cells only.

### ds_grid_get_max

`ds_grid_get_max(id, x1, y1, x2, y2)`

Returns the maximum value in a region; numeric cells only.

### ds_grid_get_min

`ds_grid_get_min(id, x1, y1, x2, y2)`

Returns the minimum value in a region; numeric cells only.

### ds_grid_get_mean

`ds_grid_get_mean(id, x1, y1, x2, y2)`

Returns the mean of the cells in a region; numeric cells only.

### ds_grid_get_disk_sum

`ds_grid_get_disk_sum(id, xcenter, ycenter, r)`

Returns the sum of the cells inside a disk.

### ds_grid_get_disk_min

`ds_grid_get_disk_min(id, xcenter, ycenter, r)`

Returns the minimum of the cells inside a disk.

### ds_grid_get_disk_max

`ds_grid_get_disk_max(id, xcenter, ycenter, r)`

Returns the maximum of the cells inside a disk.

### ds_grid_get_disk_mean

`ds_grid_get_disk_mean(id, xcenter, ycenter, r)`

Returns the mean of the cells inside a disk.

### ds_grid_value_exists

`ds_grid_value_exists(id, x1, y1, x2, y2, value)`

Returns whether the value occurs anywhere in the region.

### ds_grid_value_x

`ds_grid_value_x(id, x1, y1, x2, y2, value)`

Returns the x coordinate of a cell in the region holding the value.

### ds_grid_value_y

`ds_grid_value_y(id, x1, y1, x2, y2, value)`

Returns the y coordinate of a cell in the region holding the value.

### ds_grid_value_disk_exists

`ds_grid_value_disk_exists(id, xcenter, ycenter, r, value)`

Returns whether the value occurs anywhere in the disk.

### ds_grid_value_disk_x

`ds_grid_value_disk_x(id, xcenter, ycenter, r, value)`

Returns the x coordinate of a cell in the disk holding the value.

### ds_grid_value_disk_y

`ds_grid_value_disk_y(id, xcenter, ycenter, r, value)`

Returns the y coordinate of a cell in the disk holding the value.

### ds_grid_shuffle

`ds_grid_shuffle(id)`

Randomly reorders the values in the grid.

### ds_grid_write

`ds_grid_write(id)`

Serializes the grid to a string that can be saved, e.g. to a file.

### ds_grid_read

`ds_grid_read(id, str)`

Restores the grid from a string produced by the corresponding write function.
