# Game information

**Game information** is a help text for the players of your game: how to play, the
controls, the story, the credits. Players see it when they press **F1** in the game
(if that key is enabled in the [game settings](game-settings.md#other)), or when the
game shows it with the **Show Info** action or the `show_info()` function.

Open it by double-clicking **Game Information** in the resource tree, with
**Resources > Change Game Information**, or with the toolbar button.

## Editing the text

The form shows the text in a large box. Type your text there. The **Background
color** box next to the Options button sets the colour behind the text.

The text is stored as a rich-text (RTF) document, which can contain fonts, colours
and formatting. Instance edits it as plain text: when you open game information made
elsewhere with formatting, you see the text without the formatting. If you don't
change the text, the original document, formatting included, is kept exactly. If you
do change it, the text is saved with the document's original font and colour
settings, but formatting within the text is lost.

## Options

Click **Options** to show the window settings:

| Setting | What it does |
|---|---|
| Show help in a separate window | Shows the information in a window of its own. Otherwise it's shown inside the game window. |
| Caption | The title of the separate window. |
| Left, Top, Width, Height | Position and size of the separate window. |
| Show the window border and caption | Gives the separate window a normal border and title bar. |
| Allow the player to resize the window | Lets the player change the window size. |
| Stay always on top | Keeps the window above other windows. |
| Stop the game while showing help | Pauses the game while the information is visible. |

The settings in the middle only apply to a separate window.

> **Note:** The Instance runner currently shows the game information as plain text in
> a message box, and waits until the player closes it. The window options are kept
> with the project but don't change how it's shown yet.

Press **OK** to apply your changes, or **Cancel** to discard them.

> **Tip:** Keep the information short and practical. The first thing players want to
> know is how to control the game.

See also: [Global game settings](game-settings.md) · [Splash screens and messages](../reference/splash-and-messages.md)
