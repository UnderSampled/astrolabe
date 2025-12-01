// Quick debug script to examine GF format
using System;
using System.IO;
using System.Text;
using Astrolabe.Core.FileFormats;

Encoding.RegisterProvider(CodePagesEncodingProvider.Instance);

var cntPath = args[0];
var cnt = new CntReader(cntPath);

Console.WriteLine($"Files: {cnt.FileCount}");

// Get first file
var file = cnt.Files[0];
Console.WriteLine($"\nFirst file: {file.FullPath} ({file.FileSize} bytes)");

var data = cnt.ExtractFile(file);
Console.WriteLine($"Extracted size: {data.Length} bytes");

// Hex dump first 64 bytes
Console.WriteLine("\nFirst 64 bytes:");
for (int i = 0; i < Math.Min(64, data.Length); i++)
{
    Console.Write($"{data[i]:X2} ");
    if ((i + 1) % 16 == 0) Console.WriteLine();
}
Console.WriteLine();

// Try to parse as Montreal GF
Console.WriteLine("\nParsing as Montreal GF:");
using var reader = new BinaryReader(new MemoryStream(data));

byte version = reader.ReadByte();
Console.WriteLine($"Version: {version}");

int width = reader.ReadInt32();
int height = reader.ReadInt32();
Console.WriteLine($"Dimensions: {width} x {height}");

byte channels = reader.ReadByte();
Console.WriteLine($"Channels: {channels}");

byte mipmaps = reader.ReadByte();
Console.WriteLine($"Mipmaps: {mipmaps}");

byte repeatByte = reader.ReadByte();
Console.WriteLine($"RepeatByte: 0x{repeatByte:X2}");

ushort paletteLength = reader.ReadUInt16();
byte paletteBytesPerColor = reader.ReadByte();
Console.WriteLine($"Palette: {paletteLength} entries x {paletteBytesPerColor} bytes");

byte byte_0F = reader.ReadByte();
byte byte_10 = reader.ReadByte();
byte byte_11 = reader.ReadByte();
uint uint_12 = reader.ReadUInt32();
Console.WriteLine($"Unknown: 0x{byte_0F:X2} 0x{byte_10:X2} 0x{byte_11:X2} 0x{uint_12:X8}");

int pixelCount = reader.ReadInt32();
Console.WriteLine($"PixelCount: {pixelCount}");

byte montrealType = reader.ReadByte();
Console.WriteLine($"MontrealType: {montrealType}");

Console.WriteLine($"\nRemaining bytes after header: {data.Length - reader.BaseStream.Position}");
