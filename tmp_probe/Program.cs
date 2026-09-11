using System.Linq;
using Astrolabe.Core.FileFormats;
var r = new RelocationTableReader("disc/Gamedata/World/Levels/astrolabe/astrolabe.rtp");
var b = r.PointerBlocks.Single(x => x.Module==5 && x.Id==1);
var dupes = b.Pointers.GroupBy(p => p.OffsetInMemory).Where(g => g.Count()>1);
System.Console.WriteLine("dupes "+dupes.Count());
foreach(var g in dupes) System.Console.WriteLine("0x"+g.Key.ToString("X8")+" count="+g.Count());
// check GPT for duplicate values at different offsets
var gpt = System.IO.File.ReadAllBytes("output/test-rete/astrolabe/files/astrolabe.gpt");
var target = 0x01BDED28u;
for(int i=0;i<=gpt.Length-4;i+=4){
  var v = System.Buffers.Binary.BinaryPrimitives.ReadUInt32LittleEndian(gpt.AsSpan(i,4));
  if(v==target) System.Console.WriteLine("gpt offset 0x"+i.ToString("X")+" value 0x"+v.ToString("X8"));
}
