using System.Text;
using System.Text.Json;
using Astrolabe.Core.FileFormats.AI;
using Astrolabe.Core.FileFormats.Perso;
using Astrolabe.Core.Hub;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;
using Xunit;

namespace Astrolabe.Core.Tests;

/// <summary>
/// Codec-level round-trips for field-promoted backlog kinds (named JSON fields, not opaque shells).
/// </summary>
public sealed class FieldPromotionCodecTests
{
    [Fact]
    public void Registry_RegistersAllBacklogKinds_AsNonOpaque()
    {
        string[] kinds =
        [
            "objecttypeentry", "objecttypename", "alwayssuperobjects",
            "actiontable", "actiontree", "animhierarchies",
            "behaviorlist_normal", "behaviorlist_reflex",
            "behaviors_normal", "behaviors_reflex",
            "script", "dsgvar", "dsgmem",
            "sectorname", "sectorcollidegeo", "sectorcollideverts",
            "collidezdxlist", "collidezddlist", "collidezdelist",
            "collidezddzone", "collidezdezone", "dynam"
        ];

        foreach (var kind in kinds)
        {
            Assert.True(StructCodecRegistry.TryGet(kind, out var binding), $"missing codec for {kind}");
            Assert.False(
                binding.UsesExternalBinaryPayload,
                $"{kind} must not be OpaqueBinaryRecord-only");
        }
    }

    [Fact]
    public void ObjectTypeEntry_RoundTripsWireAndJsonFields()
    {
        var wire = new byte[ObjectTypeEntryCodec.Size];
        // next=0, prev=0x11111111, hdr=0x22222222, name=0x33333333, marker=1
        BitConverter.GetBytes(0x11111111).CopyTo(wire, 0x04);
        BitConverter.GetBytes(0x22222222).CopyTo(wire, 0x08);
        BitConverter.GetBytes(0x33333333).CopyTo(wire, 0x0C);
        BitConverter.GetBytes(1).CopyTo(wire, 0x10);

        var record = ObjectTypeEntryCodec.Instance.Read(wire, 0, wire.Length);
        Assert.Equal(1u, record.Marker);
        Assert.Equal(0x33333333, record.Name.WireValue);

        var written = ObjectTypeEntryCodec.Instance.Write(record);
        Assert.Equal(wire, written);

        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            ObjectTypeEntryCodec.Instance.ToJson(record, writer);
        }

        using var doc = JsonDocument.Parse(stream.ToArray());
        Assert.True(doc.RootElement.TryGetProperty("marker", out var marker));
        Assert.Equal(1, marker.GetInt32());
        Assert.True(doc.RootElement.TryGetProperty("name", out _));
        Assert.False(doc.RootElement.TryGetProperty("data", out _));
        Assert.False(doc.RootElement.TryGetProperty("path", out _));
    }

    [Fact]
    public void ObjectTypeName_PreservesCStringAsTextAndBytes()
    {
        var wire = Encoding.ASCII.GetBytes("wanted\0");
        var record = TextNameCodec.ObjectTypeName.Read(wire, 0, wire.Length);
        Assert.Equal("wanted", record.Text);
        Assert.Equal(wire, record.Bytes);

        var written = TextNameCodec.ObjectTypeName.Write(record);
        Assert.Equal(wire, written);

        // Editor path: text only rebuilds C-string.
        var fromText = new TextNameRecord { Schema = TextNameCodec.ObjectTypeName.Schema, Type = "objecttypename", Text = "wanted" };
        Assert.Equal(wire, TextNameCodec.ObjectTypeName.Write(fromText));
    }

    [Fact]
    public void DsgVar_And_DsgMem_RoundTripNamedFields()
    {
        var dsgVar = new byte[DsgVarCodec.Size];
        BitConverter.GetBytes(0x100).CopyTo(dsgVar, 0x00);
        BitConverter.GetBytes(0x200).CopyTo(dsgVar, 0x04);
        BitConverter.GetBytes(0x1Fu).CopyTo(dsgVar, 0x08);
        BitConverter.GetBytes(0x0Bu).CopyTo(dsgVar, 0x0C);

        var varRec = DsgVarCodec.Instance.Read(dsgVar, 0, dsgVar.Length);
        Assert.Equal(0x1Fu, varRec.DsgMemBufferLength);
        Assert.Equal(0x0Bu, varRec.AmountOfInfos);
        Assert.Equal(dsgVar, DsgVarCodec.Instance.Write(varRec));

        var dsgMem = new byte[DsgMemCodec.Size];
        BitConverter.GetBytes(0x300).CopyTo(dsgMem, 0x00);
        BitConverter.GetBytes(0x400).CopyTo(dsgMem, 0x04);
        BitConverter.GetBytes(0x500).CopyTo(dsgMem, 0x08);
        var memRec = DsgMemCodec.Instance.Read(dsgMem, 0, dsgMem.Length);
        Assert.Equal(dsgMem, DsgMemCodec.Instance.Write(memRec));
    }

    [Fact]
    public void Script_NodeStream_RoundTripsFieldLevelNodes()
    {
        // keyword indent=1 + end marker indent=0
        var wire = new byte[]
        {
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08
        };

        var record = ScriptBlobCodec.Instance.Read(wire, 0, wire.Length);
        Assert.Equal("nodes", record.Form);
        Assert.Equal(2, record.Nodes.Length);
        Assert.Equal(1, record.Nodes[0].Indent);
        Assert.Equal(0, record.Nodes[1].Indent);
        Assert.Equal(wire, ScriptBlobCodec.Instance.Write(record));
    }

    [Fact]
    public void Script_Shell_RoundTripsOffScriptPointer()
    {
        var wire = new byte[8];
        BitConverter.GetBytes(0x09000d58).CopyTo(wire, 0);
        BitConverter.GetBytes(0x000006a1u).CopyTo(wire, 4);

        var record = ScriptBlobCodec.Instance.Read(wire, 0, wire.Length);
        Assert.Equal("shell", record.Form);
        Assert.Equal(0x09000d58, record.OffScript.WireValue);
        Assert.Equal(wire, ScriptBlobCodec.Instance.Write(record));
    }

    [Fact]
    public void BehaviorArray_RoundTripsEntriesWithNamedPointers()
    {
        var wire = new byte[BehaviorArrayCodec.EntrySize * 2];
        BitConverter.GetBytes(0x1111).CopyTo(wire, 0x00);
        BitConverter.GetBytes(0x2222).CopyTo(wire, 0x04);
        BitConverter.GetBytes(2u).CopyTo(wire, 0x08);
        BitConverter.GetBytes(0x3333).CopyTo(wire, BehaviorArrayCodec.EntrySize);

        var record = BehaviorArrayCodec.BehaviorsNormal.Read(wire, 0, wire.Length);
        Assert.Equal(2, record.Entries.Length);
        Assert.Equal(0x1111, record.Entries[0].Scripts.WireValue);
        Assert.Equal(0x2222, record.Entries[0].ScheduleScript.WireValue);
        Assert.Equal(2u, record.Entries[0].Unknown08);
        Assert.Equal(wire, BehaviorArrayCodec.BehaviorsNormal.Write(record));
    }

    [Fact]
    public void AlwaysSuperObjects_RoundTripsSuperObjectEntries()
    {
        var so = new byte[SuperObjectCodec.Size];
        BitConverter.GetBytes(4u).CopyTo(so, 0x00); // type code
        BitConverter.GetBytes(0xABCDEF01).CopyTo(so, 0x04); // offData

        var record = AlwaysSuperObjectsCodec.Instance.Read(so, 0, so.Length);
        Assert.Single(record.Entries);
        Assert.Equal(4u, record.Entries[0].TypeCode);
        Assert.Equal(unchecked((int)0xABCDEF01), record.Entries[0].OffData.WireValue);
        Assert.Equal(so, AlwaysSuperObjectsCodec.Instance.Write(record));
    }

    [Fact]
    public void AlwaysSuperObjects_NestedUriRewrite_SkipsTypeCodeAndChildrenCount()
    {
        var names = AlwaysSuperObjectsCodec.Instance.EntryPointerFields
            .Select(f => f.Name)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
        Assert.DoesNotContain("typeCode", names);
        Assert.DoesNotContain("childrenCount", names);
        Assert.Contains("offData", names);
        Assert.Contains("matrix", names);
    }

    [Fact]
    public void AnimHierarchies_RoundTripsChannelPairs()
    {
        // 4-byte pairs only
        var wire = new byte[] { 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00 };
        var record = AnimHierarchiesCodec.Instance.Read(wire, 0, wire.Length);
        Assert.Equal(2, record.Entries.Length);
        Assert.Equal(1, record.Entries[0].ChildChannelId);
        Assert.Equal(2, record.Entries[0].ParentChannelId);
        Assert.Equal(wire, AnimHierarchiesCodec.Instance.Write(record));
    }

    [Fact]
    public void Dynam_And_CollideZones_RoundTrip()
    {
        var dynam = new byte[DynamCodec.Size];
        BitConverter.GetBytes(1u).CopyTo(dynam, 0x00);
        BitConverter.GetBytes(0x1010).CopyTo(dynam, 0x10);
        BitConverter.GetBytes(0x2020).CopyTo(dynam, 0x14);
        var dRec = DynamCodec.Instance.Read(dynam, 0, dynam.Length);
        Assert.Equal(dynam, DynamCodec.Instance.Write(dRec));

        var list = new byte[CollideZoneListCodec.Size];
        BitConverter.GetBytes(0x10).CopyTo(list, 0x00);
        BitConverter.GetBytes(0x20).CopyTo(list, 0x04);
        BitConverter.GetBytes(3u).CopyTo(list, 0x08);
        Assert.Equal(list, CollideZoneListCodec.Zdx.Write(CollideZoneListCodec.Zdx.Read(list, 0, list.Length)));

        var zone = new byte[CollideZoneCodec.Size];
        BitConverter.GetBytes(0x30).CopyTo(zone, 0x00);
        BitConverter.GetBytes(0x40).CopyTo(zone, 0x04);
        BitConverter.GetBytes(0x50).CopyTo(zone, 0x08);
        for (var i = 0x0C; i < zone.Length; i++)
        {
            zone[i] = (byte)i;
        }

        Assert.Equal(zone, CollideZoneCodec.Zdd.Write(CollideZoneCodec.Zdd.Read(zone, 0, zone.Length)));
    }

    [Fact]
    public void SectorCollideGeo_And_Verts_RoundTrip()
    {
        var geo = new byte[SectorCollideGeoCodec.Size];
        BitConverter.GetBytes(0x13u).CopyTo(geo, 0x00);
        BitConverter.GetBytes(0x100).CopyTo(geo, 0x04);
        BitConverter.GetBytes(0x200).CopyTo(geo, 0x08);
        BitConverter.GetBytes(4u).CopyTo(geo, 0x14);
        BitConverter.GetBytes(0x300).CopyTo(geo, 0x18);
        BitConverter.GetBytes(0x400).CopyTo(geo, 0x1C);
        Assert.Equal(geo, SectorCollideGeoCodec.Instance.Write(
            SectorCollideGeoCodec.Instance.Read(geo, 0, geo.Length)));

        var verts = new byte[24];
        BitConverter.GetBytes(1.0f).CopyTo(verts, 0);
        BitConverter.GetBytes(2.0f).CopyTo(verts, 4);
        BitConverter.GetBytes(3.0f).CopyTo(verts, 8);
        BitConverter.GetBytes(4.0f).CopyTo(verts, 12);
        BitConverter.GetBytes(5.0f).CopyTo(verts, 16);
        BitConverter.GetBytes(6.0f).CopyTo(verts, 20);
        var rec = Float3ArrayCodec.SectorCollideVerts.Read(verts, 0, verts.Length);
        Assert.Equal(2, rec.Values.Length);
        Assert.Equal(verts, Float3ArrayCodec.SectorCollideVerts.Write(rec));
    }

    [Fact]
    public void ActionTable_And_ActionTree_RoundTrip()
    {
        var table = new byte[ActionTableCodec.Size];
        BitConverter.GetBytes(0xAABB).CopyTo(table, 0x00);
        BitConverter.GetBytes(3u).CopyTo(table, 0x04);
        Assert.Equal(table, ActionTableCodec.Instance.Write(
            ActionTableCodec.Instance.Read(table, 0, table.Length)));

        var tree = new byte[ActionTreeCodec.Size];
        BitConverter.GetBytes(1u).CopyTo(tree, 0x00);
        BitConverter.GetBytes(0x10).CopyTo(tree, 0x08);
        BitConverter.GetBytes(0x20).CopyTo(tree, 0x10);
        BitConverter.GetBytes(0x30).CopyTo(tree, 0x18);
        Assert.Equal(tree, ActionTreeCodec.Instance.Write(
            ActionTreeCodec.Instance.Read(tree, 0, tree.Length)));
    }
}
