using Astrolabe.Core.Serialization;

namespace Astrolabe.Core.Hub;

internal static class HubReferenceIO
{
    public static HubReference Read(ReadOnlySpan<byte> data, int offset) =>
        HubReference.FromWire(StructBinaryIO.ReadInt32(data, offset));

    public static void Write(Span<byte> data, int offset, HubReference? reference) =>
        StructBinaryIO.WriteInt32(data, offset, Materialize(reference));

    public static int Materialize(HubReference? reference) =>
        reference == null || reference.IsNull ? 0 : reference.MaterializeForWire();
}