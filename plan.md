Okay, Here is the project: We're going to make tools to import game data (3D models, maps, etc.) from the 1990s game "Hype: the Time Quest" into Godot. We will use C# to generate intermediate OpenSpace data, Godot scene files for maps, Godot-native mesh resources, and GDScript for interactions where needed.

The game was made by Ubisoft for Brandstätter Group (who makes Playmobil), under the name 'Playmobil Interactive'. It used the game engine from "Tonic Trouble" and Rayman 2, OpenSpace. The single most important resource we have available to us is Raymap, made by the Rayman Community, which is able to read much of the date (maps, models, state graphs, etc.) as a library for Unity, and display it in that engine. The code for that is here: https://github.com/byvar/raymap. I have imported it as a git submodule to reference it during development, located at /reference/raymap.

Raymap has a dependency on https://github.com/BinarySerializer/BinarySerializer.OpenSpace (and https://github.com/BinarySerializer/BinarySerializer). We should also be able to use these libraries, and any others Raymap uses. Much of it's code can also be reused, though it is important that no Unity-specific code is added to this project.

## Game Files

Due to copyright concerns, any user will have to bring their own copy of the original game in order to use any of the assets. Astrolabe should work from a mounted ISO directory or pre-extracted files, then convert from there.

Use a mounted or pre-extracted copy of the Hype disc for local testing.

## First task

Let's start by editing the README, and setting up the basic project structure. Set it up so I can point the tool at mounted or pre-extracted files; we'll move on from there to porting Raymap concepts and importing meshes with textures. The last step (both in development, and for the actions the tool takes) will then be to generate the Godot scene files and capture the state graphs and interactions -- this will be a todo item for now.

Think ahead for what architecture makes sense so that we will be able to easily review code, fix bugs, and package an "installer" that anyone can use to extract the data needed to run the game in whichever Godot-based engine someone might make using the extracted data.

## Second task

Read through the raymap code and write professional-quality documentation for each of the file formats we will expect to see in the game files. I imagine this would be things like the asset archive format, the mesh format, scene description format, etc. 

## Third Task

Implement the conversion tool up to the point where extracted meshes are written as Godot-native mesh resources and load in a generated Godot project. You have succeeded once the mesh and texture data are visible in Godot from the generated project.
