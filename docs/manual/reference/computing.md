# Computing

Functions for numbers, strings, dates and times, script arguments and the special instance references.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Special instance references](#special-instance-references)
- [Script arguments](#script-arguments)
- [Basic constants](#basic-constants)
- [Real-valued functions](#real-valued-functions)
- [String handling](#string-handling)
- [Dates and times](#dates-and-times)

## Special instance references

Values that stand for particular instances wherever an instance id is expected.

### Constants: special instance references

| Constant | Value | Meaning |
| --- | --- | --- |
| `self` | -1 | Special instance reference meaning the instance currently executing the code. |
| `other` | -2 | Special instance reference to the other instance involved in a collision event. |
| `all` | -3 | Special reference meaning every instance of every object. |
| `noone` | -4 | Special reference meaning no instance at all; returned when nothing is found. |
| `global` | -5 | Prefix for variables shared by the whole game rather than belonging to one instance. |
| `local` | -7 | Special value listed alongside self, other and global; its meaning is not documented. |

## Script arguments

The arguments a script receives from its caller.

### argument_relative

*Script variable, read-only.* Flag set when a script is called from a relative-enabled action, indicating the action's relative option.

### argument

*Script variable, array, indices 0 to 15.* Array-style access to the arguments passed to the current script, as argument\[n\].

### argument0

*Script variable.* Script argument number 0 (argument0..argument15, at most 16 per call).

### argument1

*Script variable.* Script argument number 1 (argument0..argument15, at most 16 per call).

### argument2

*Script variable.* Script argument number 2 (argument0..argument15, at most 16 per call).

### argument3

*Script variable.* Script argument number 3 (argument0..argument15, at most 16 per call).

### argument4

*Script variable.* Script argument number 4 (argument0..argument15, at most 16 per call).

### argument5

*Script variable.* Script argument number 5 (argument0..argument15, at most 16 per call).

### argument6

*Script variable.* Script argument number 6 (argument0..argument15, at most 16 per call).

### argument7

*Script variable.* Script argument number 7 (argument0..argument15, at most 16 per call).

### argument8

*Script variable.* Script argument number 8 (argument0..argument15, at most 16 per call).

### argument9

*Script variable.* Script argument number 9 (argument0..argument15, at most 16 per call).

### argument10

*Script variable.* Script argument number 10 (argument0..argument15, at most 16 per call).

### argument11

*Script variable.* Script argument number 11 (argument0..argument15, at most 16 per call).

### argument12

*Script variable.* Script argument number 12 (argument0..argument15, at most 16 per call).

### argument13

*Script variable.* Script argument number 13 (argument0..argument15, at most 16 per call).

### argument14

*Script variable.* Script argument number 14 (argument0..argument15, at most 16 per call).

### argument15

*Script variable.* Script argument number 15 (argument0..argument15, at most 16 per call).

### argument_count

*Script variable.* Number of arguments actually passed to the currently running script.

## Basic constants

Truth values and the number pi.

### Constants: basic constants

| Constant | Value | Meaning |
| --- | --- | --- |
| `true` | 1 | Constant equal to 1. |
| `false` | 0 | Constant equal to 0. |
| `pi` | 3.141592653589793 | Constant equal to 3.1415..., the ratio of circumference to diameter. |

## Real-valued functions

Arithmetic, rounding, random numbers, trigonometry and other functions on real numbers.

### is_real

`is_real(value)`

Returns whether the value is a real number rather than a string.

### is_string

`is_string(value)`

Returns whether the value is a string rather than a real number.

### random

`random(x)`

Returns a random real from 0 up to but not including x.

### random_range

`random_range(x1, x2)`

Returns a random real from x1 (inclusive) up to x2 (exclusive).

### irandom

`irandom(x)`

Returns a random integer between 0 and x, inclusive of x when x is an integer.

### irandom_range

`irandom_range(x1, x2)`

Returns a random integer between x1 and x2 inclusive; non-integer arguments are rounded down.

### random_set_seed

`random_set_seed(seed)`

Sets the integer seed of the random generator so sequences can be repeated.

### random_get_seed

`random_get_seed()`

Returns the current random seed.

### randomize

`randomize()`

Sets the random seed to a randomly chosen value.

### choose

`choose(x1, x2, x3, ...)`

Returns one of its arguments picked at random; accepts up to 16 arguments.

### abs

`abs(x)`

Returns the absolute value of x.

### round

`round(x)`

Returns x rounded to the nearest integer.

### floor

`floor(x)`

Returns x rounded down to an integer.

### ceil

`ceil(x)`

Returns x rounded up to an integer.

### sign

`sign(x)`

Returns -1, 0 or 1 according to the sign of x.

### frac

`frac(x)`

Returns the fractional part of x.

### sqrt

`sqrt(x)`

Returns the square root of x, which must be non-negative.

### sqr

`sqr(x)`

Returns x multiplied by itself.

### exp

`exp(x)`

Returns e raised to the power x.

### ln

`ln(x)`

Returns the natural logarithm of x.

### log2

`log2(x)`

Returns the base-2 logarithm of x.

### log10

`log10(x)`

Returns the base-10 logarithm of x.

### sin

`sin(x)`

Returns the sine of x, with x in radians.

### cos

`cos(x)`

Returns the cosine of x, with x in radians.

### tan

`tan(x)`

Returns the tangent of x, with x in radians.

### arcsin

`arcsin(x)`

Returns the inverse sine of x, in radians.

### arccos

`arccos(x)`

Returns the inverse cosine of x, in radians.

### arctan

`arctan(x)`

Returns the inverse tangent of x, in radians.

### arctan2

`arctan2(y, x)`

Returns the arctangent of y/x as an angle in the correct quadrant.

### degtorad

`degtorad(x)`

Converts an angle from degrees to radians.

### radtodeg

`radtodeg(x)`

Converts an angle from radians to degrees.

### power

`power(x, n)`

Returns x raised to the power n.

### logn

`logn(n, x)`

Returns the base-n logarithm of x.

### min

`min(x1, x2, x3, ...)`

Returns the smallest argument; up to 16 arguments, all real or all strings.

### max

`max(x1, x2, x3, ...)`

Returns the largest argument; up to 16 arguments, all real or all strings.

### mean

`mean(x1, x2, x3, ...)`

Returns the average of up to 16 real arguments.

### median

`median(x1, x2, x3, ...)`

Returns the middle value of up to 16 reals; for an even count, the lower middle value.

### clamp

`clamp(value, min, max)`

Returns value limited to the range from min to max.

### lerp

`lerp(val1, val2, amount)`

Linearly interpolates between val1 and val2 by the fraction amount.

### dot_product

`dot_product(x1, y1, x2, y2)`

Returns the dot product of the two 2D vectors after normalizing both to unit length.

### dot_product_3d

`dot_product_3d(x1, y1, z1, x2, y2, z2)`

Returns the dot product of the two 3D vectors after normalizing both to unit length.

### point_distance_3d

`point_distance_3d(x1, y1, z1, x2, y2, z2)`

Returns the distance between two 3D points.

### point_distance

`point_distance(x1, y1, x2, y2)`

Returns the distance between two 2D points.

### point_direction

`point_direction(x1, y1, x2, y2)`

Returns the direction in degrees from the first point toward the second.

### lengthdir_x

`lengthdir_x(len, direction)`

Returns the horizontal component of a vector with the given length and direction.

### lengthdir_y

`lengthdir_y(len, direction)`

Returns the vertical component of a vector with the given length and direction.

## String handling

Building, searching, cutting and converting strings.

### real

`real(str)`

Converts a string to a real number; may include minus sign, decimal dot and exponent.

### string

`string(value)`

Converts a real to a string: no decimals for integers, otherwise two decimal places.

### string_format

`string_format(value, total, dec)`

Converts value to a string using a minimum total width and a fixed number of decimals.

### chr

`chr(value)`

Returns a one-character string for the character code value in the current font codepage.

### ansi_char

`ansi_char(value)`

Returns a one-character string made from the raw byte value value.

### ord

`ord(char)`

Returns the character code of the first character of the string.

### string_length

`string_length(str)`

Returns the number of characters in the string.

### string_pos

`string_pos(substr, str)`

Returns the 1-based position of substr within str, or 0 if absent.

### string_copy

`string_copy(str, index, count)`

Returns the substring of str starting at index (1-based) with count characters.

### string_char_at

`string_char_at(str, index)`

Returns the single character of str at the 1-based position index.

### string_delete

`string_delete(str, index, count)`

Returns str with count characters removed starting at index.

### string_insert

`string_insert(substr, str, index)`

Returns str with substr inserted at position index.

### string_lower

`string_lower(str)`

Returns a lowercase copy of the string.

### string_upper

`string_upper(str)`

Returns an uppercase copy of the string.

### string_repeat

`string_repeat(str, count)`

Returns the string repeated count times.

### string_letters

`string_letters(str)`

Returns only the letters contained in str.

### string_digits

`string_digits(str)`

Returns only the digits contained in str.

### string_lettersdigits

`string_lettersdigits(str)`

Returns only the letters and digits contained in str.

### string_replace

`string_replace(str, substr, newstr)`

Gives a copy of str in which only the first match of substr is swapped for newstr.

### string_replace_all

`string_replace_all(str, substr, newstr)`

Returns str with every occurrence of substr replaced by newstr.

### string_count

`string_count(substr, str)`

Returns how many times substr occurs in str.

### clipboard_has_text

`clipboard_has_text()`

Returns whether the clipboard currently holds text.

### clipboard_set_text

`clipboard_set_text(str)`

Puts the given string onto the clipboard.

### clipboard_get_text

`clipboard_get_text()`

Returns the text currently on the clipboard.

## Dates and times

Date-time values: creating them, reading their parts, comparing and adding to them.

### date_current_datetime

`date_current_datetime()`

Returns the date-time value for the present moment.

### date_current_date

`date_current_date()`

Returns the date-time value for today's date, ignoring the time.

### date_current_time

`date_current_time()`

Returns the date-time value for the current time of day, ignoring the date.

### date_create_datetime

`date_create_datetime(year, month, day, hour, minute, second)`

Builds a date-time value from the given date and time parts.

### date_create_date

`date_create_date(year, month, day)`

Builds a date-time value from the given date.

### date_create_time

`date_create_time(hour, minute, second)`

Builds a date-time value from the given time of day.

### date_valid_datetime

`date_valid_datetime(year, month, day, hour, minute, second)`

Returns whether the given date and time are a valid combination.

### date_valid_date

`date_valid_date(year, month, day)`

Returns whether the given date is valid.

### date_valid_time

`date_valid_time(hour, minute, second)`

Returns whether the given time is valid.

### date_inc_year

`date_inc_year(date, amount)`

Returns a date shifted forward by an integer number of years.

### date_inc_month

`date_inc_month(date, amount)`

Returns a date shifted forward by an integer number of months.

### date_inc_week

`date_inc_week(date, amount)`

Returns a date shifted forward by an integer number of weeks.

### date_inc_day

`date_inc_day(date, amount)`

Returns a date shifted forward by an integer number of days.

### date_inc_hour

`date_inc_hour(date, amount)`

Returns a date shifted forward by an integer number of hours.

### date_inc_minute

`date_inc_minute(date, amount)`

Returns a date shifted forward by an integer number of minutes.

### date_inc_second

`date_inc_second(date, amount)`

Returns a date shifted forward by an integer number of seconds.

### date_get_year

`date_get_year(date)`

Returns the year component of the given date.

### date_get_month

`date_get_month(date)`

Returns the month component of the given date.

### date_get_week

`date_get_week(date)`

Returns the week component of the given date.

### date_get_day

`date_get_day(date)`

Returns the day component of the given date.

### date_get_hour

`date_get_hour(date)`

Returns the hour component of the given date.

### date_get_minute

`date_get_minute(date)`

Returns the minute component of the given date.

### date_get_second

`date_get_second(date)`

Returns the second component of the given date.

### date_get_weekday

`date_get_weekday(date)`

Returns the day of the week for the date.

### date_get_day_of_year

`date_get_day_of_year(date)`

Returns the day count within the year for the date.

### date_get_hour_of_year

`date_get_hour_of_year(date)`

Returns the hour count within the year for the date.

### date_get_minute_of_year

`date_get_minute_of_year(date)`

Returns the minute count within the year for the date.

### date_get_second_of_year

`date_get_second_of_year(date)`

Returns the second count within the year for the date.

### date_year_span

`date_year_span(date1, date2)`

Returns the number of years between two dates, with partial units as fractions.

### date_month_span

`date_month_span(date1, date2)`

Returns the number of months between two dates, with partial units as fractions.

### date_week_span

`date_week_span(date1, date2)`

Returns the number of weeks between two dates, with partial units as fractions.

### date_day_span

`date_day_span(date1, date2)`

Returns the number of days between two dates, with partial units as fractions.

### date_hour_span

`date_hour_span(date1, date2)`

Returns the number of hours between two dates, with partial units as fractions.

### date_minute_span

`date_minute_span(date1, date2)`

Returns the number of minutes between two dates, with partial units as fractions.

### date_second_span

`date_second_span(date1, date2)`

Returns the number of seconds between two dates, with partial units as fractions.

### date_compare_datetime

`date_compare_datetime(date1, date2)`

Compares two date-times using the full date and time; returns -1, 0 or 1.

### date_compare_date

`date_compare_date(date1, date2)`

Compares two date-times using the date part only; returns -1, 0 or 1.

### date_compare_time

`date_compare_time(date1, date2)`

Compares two date-times using the time part only; returns -1, 0 or 1.

### date_date_of

`date_date_of(date)`

Returns the date part of a date-time, with the time set to 0.

### date_time_of

`date_time_of(date)`

Returns the time part of a date-time, with the date set to 0.

### date_datetime_string

`date_datetime_string(date)`

Returns the full date and time formatted as a string in the system default format.

### date_date_string

`date_date_string(date)`

Returns the date part only formatted as a string in the system default format.

### date_time_string

`date_time_string(date)`

Returns the time part only formatted as a string in the system default format.

### date_days_in_month

`date_days_in_month(date)`

Returns the number of days in the month of the date.

### date_days_in_year

`date_days_in_year(date)`

Returns the number of days in the year of the date.

### date_leap_year

`date_leap_year(date)`

Returns whether the date's year is a leap year.

### date_is_today

`date_is_today(date)`

Returns whether the date falls on the current day.
