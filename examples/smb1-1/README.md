# SMB1-1 example

A rough recreation of a classic level from a classic game.

## Notes

There are a number of things we can note about the implementation which might be relevant to someone
building a game using Heavy. Here's a short summary of the important things we came across and did
while building it.

### Sprite padding

Sprites here are done using Aseprite and generating packed Aseprite JSON files. There are a few
gotchas; due to floating point error, it is possible for the "next pixel over" from one frame of a
sprite to be briefly visible. This is a pretty common issue w/ texture atlases known as "bleed". For
this reason, the spritesheets are generated with one pixel of padding. This causes Aseprite to pack
with at minimum one pixel of transparency between each frame of the sprite, and eliminates the pixel
bleed problem.