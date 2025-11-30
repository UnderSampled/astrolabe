Okay, Here is the project: We're going to make tools to import game data (3D models, maps, etc.) from the 1990s game "Hype: the Time Quest" into Godot.  We will be using Python with good type hints for most of the conversion scripts, to generate Godot scene files for maps, gdscript for interractions where needed, and gltf for 3D models.

The game was made by Ubisoft for Brandstätter Group (who makes Playmobil), under the name 'Playmobil Interactive'. It used the game engine from "Tonic Trouble" and Rayman 2, OpenSpace. The single most important resource we have available to us is Raymap, made by the Rayman Community, which is able to read much of the date (maps, models, state graphs, etc.) as a library for Unity, and display it in that engine. The code for that is here: https://github.com/byvar/raymap. I have imported it as a git submodule to reference it during development, located at /reference/raymap.

Raymap has a dependency on https://github.com/BinarySerializer/BinarySerializer.OpenSpace (and https://github.com/BinarySerializer/BinarySerializer). We should also be able to use these libraries, and any others Raymap uses. Much of it's code can also be reused, though it is important that no Unity-specific code is added to this project.

## Game Files

Do to copyright concerns, any user will have to bring their own copy of the original game iso in order to use any of the assets. So, our scripts should start with the iso, extracting and converting it from there. This is a pattern used by several other high profile community remaster projects, such as OpenMW or Freespace Open, so I feel fairly confident that it should be sufficient to forgo any legal concerns.

I have put my own copy of the Hype ISO in /hype.iso.

## First task

Let's start by editing the README, and setting up the basic project structure. Set it up so I can drop in the iso and the script will extract it; we'll move on from there to porting raymapp and importing meshes with textures. The last step (both in development, and for the actions the script takes) will then be to generate the godot scene files and capture the state graphs and interractions -- this will be a todo item for now.

Think ahead for what architecture makes sense so that we will be able to easily review code, fix bugs, and package an "installer" that anyone can use to extract the data needed to run the game in whichever Godot-based engine someone might make using the extracted data.

## Second task

Read through the raymap code and write professional-quality documentation for each of the file formats we will expect to see in the game files. I imagine this would be things like the asset archive format, the mesh format, scene description format, etc. 

## Third Task

Implement the conversion tool up to the point where the extracted meshes can be imported as GTLF files into blender. I recommend trying to import it into blender from the command-line (e.g. blender python). I have blender installed through flatpack. You have succeeded once you are able to see the mesh and texture data loaded into blender.
