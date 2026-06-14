using System.Buffers.Binary;
using Astrolabe.Core.Serialization;
using Astrolabe.Core.Serialization.Codecs;
using Xunit;

namespace Astrolabe.Core.Tests;

public sealed class RelocationGeneratorTests
{
    [Fact]
    public void BehaviorArrayCodec_MisalignedLength_ReturnsNoPointerFields()
    {
        var fields = BehaviorArrayCodec.BehaviorsNormal.GetPointerFieldsForLength(0x11);
        Assert.Empty(fields);
    }

    [Fact]
    public void BehaviorArrayCodec_TruncatedAlignedPrefix_StillEmitsPointerFields()
    {
        IPointerArrayCodec codec = BehaviorArrayCodec.BehaviorsNormal;
        var fields = codec.EnumeratePointerFields(new byte[0x20]);
        Assert.Equal(4, fields.Count);
    }

    [Fact]
    public void AnimFramesCodec_UsesFrameStride()
    {
        Assert.Equal(0x10, AnimFramesCodec.Instance.PointerEntryStride);
    }

    [Fact]
    public void AnimFramesCodec_MisalignedLength_ReturnsNoPointerFields()
    {
        var fields = AnimFramesCodec.Instance.GetPointerFieldsForLength(0x14);
        Assert.Empty(fields);
    }

    [Fact]
    public void RawBlobCodec_OnlyEnumeratesVmLikePointers()
    {
        var data = new byte[8];
        BinaryPrimitives.WriteInt32LittleEndian(data.AsSpan(0), 0x1234);
        BinaryPrimitives.WriteInt32LittleEndian(data.AsSpan(4), 0x0900_0000);

        var fields = RawBlobCodec.Instance.EnumeratePointerFields(data);
        Assert.Single(fields);
        Assert.Equal(4, fields[0].Offset);
        Assert.True(fields[0].RequiresDecompressedTarget);
    }

    [Fact]
    public void VmPointerScanning_RejectsNonVmValues()
    {
        Assert.False(VmPointerScanning.IsLikelyVirtualAddress(0));
        Assert.False(VmPointerScanning.IsLikelyVirtualAddress(0x1234));
        Assert.True(VmPointerScanning.IsLikelyVirtualAddress(0x0900_0000));
    }
}