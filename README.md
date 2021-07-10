# Sweepers
A minesweeper implementation for the terminal.


## How to play
Minesweeper is a rather simple game for a single player. It is played on a rectangular grid. Each cell is either empty or contains a mine.
At the start of the game all cells are hidden from the player. They can click a cell to reveal its contents or mark it.
The game is won if all empty cells are revealed and all mine cells are marked. If the player reveals a mine cell, they
lose immediately. After an empty cell was revealed, it shows the number of mines that surround it. This enables the player to
deduce where mines are located by combining information of multiple revealed cells.


## Controls
| Input       | Action                             | Alternate action                                                   |
|-------------|------------------------------------|--------------------------------------------------------------------|
| left click  | reveal hidden cell                 | reveal all neighbours of revealed cell if mine count matches marks |
| right click | mark hidden cell                   | reveal all neighbours of revealed cell if mine count matches marks |
| q           | quit game                          |                                                                    |
| r           | restart game (after game finished) |                                                                    |
