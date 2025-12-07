using System.Numerics;

namespace Astrolabe.Core.FileFormats.Animation;

/// <summary>
/// Reads Montreal engine animation data from memory.
/// </summary>
public class AnimationReader
{
    private readonly MemoryContext _memory;
    private readonly Dictionary<int, AnimationMontreal> _animationCache = new();
    private readonly Dictionary<int, CompressedMatrix> _matrixCache = new();

    public AnimationReader(MemoryContext memory)
    {
        _memory = memory;
    }

    /// <summary>
    /// Reads an AnimationMontreal structure at the given address.
    /// </summary>
    public AnimationMontreal? ReadAnimation(int address)
    {
        if (address == 0) return null;
        if (_animationCache.TryGetValue(address, out var cached))
            return cached;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var anim = new AnimationMontreal { Address = address };

            // AnimationMontreal structure:
            // +0x00: Pointer off_frames
            // +0x04: byte num_frames
            // +0x05: byte speed
            // +0x06: byte num_channels
            // +0x07: byte unkbyte
            // +0x08: Pointer off_unk
            // +0x0C: uint32 (skip)
            // +0x10: uint32 (skip)
            // +0x14: Matrix4x4 speedMatrix (64 bytes)

            anim.OffFrames = reader.ReadInt32();
            anim.NumFrames = reader.ReadByte();
            anim.Speed = reader.ReadByte();
            anim.NumChannels = reader.ReadByte();
            anim.UnkByte = reader.ReadByte();
            anim.OffUnk = reader.ReadInt32();
            reader.ReadUInt32(); // skip
            reader.ReadUInt32(); // skip

            // Read speed matrix (4x4 floats)
            anim.SpeedMatrix = ReadMatrix4x4(reader);

            // Skip additional padding
            reader.ReadUInt32();
            reader.ReadUInt32();

            // Read frames
            if (anim.OffFrames != 0 && anim.NumFrames > 0)
            {
                anim.Frames = new AnimFrameMontreal[anim.NumFrames];
                for (int i = 0; i < anim.NumFrames; i++)
                {
                    // Each frame pointer is at offFrames + i * 16
                    int frameAddr = anim.OffFrames + (i * 16);
                    anim.Frames[i] = ReadFrame(frameAddr, anim.NumChannels) ?? new AnimFrameMontreal();
                }
            }

            _animationCache[address] = anim;
            return anim;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads an AnimFrameMontreal structure.
    /// </summary>
    private AnimFrameMontreal? ReadFrame(int address, byte numChannels)
    {
        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var frame = new AnimFrameMontreal { Address = address };

            // AnimFrameMontreal structure:
            // +0x00: Pointer off_channels
            // +0x04: Pointer off_mat
            // +0x08: Pointer off_vec
            // +0x0C: Pointer off_hierarchies

            frame.OffChannels = reader.ReadInt32();
            frame.OffMat = reader.ReadInt32();
            frame.OffVec = reader.ReadInt32();
            frame.OffHierarchies = reader.ReadInt32();

            // Read channels
            if (frame.OffChannels != 0 && numChannels > 0)
            {
                frame.Channels = new AnimChannelMontreal[numChannels];
                var channelPtrReader = _memory.GetReaderAt(frame.OffChannels);
                if (channelPtrReader != null)
                {
                    for (int i = 0; i < numChannels; i++)
                    {
                        int channelAddr = channelPtrReader.ReadInt32();
                        frame.Channels[i] = ReadChannel(channelAddr) ?? new AnimChannelMontreal();
                    }
                }
            }

            // Read hierarchies
            if (frame.OffHierarchies != 0)
            {
                var hierReader = _memory.GetReaderAt(frame.OffHierarchies);
                if (hierReader != null)
                {
                    uint numHierarchies = hierReader.ReadUInt32();
                    int offHierarchies2 = hierReader.ReadInt32();

                    if (offHierarchies2 != 0 && numHierarchies > 0 && numHierarchies < 256)
                    {
                        frame.Hierarchies = new AnimHierarchy[numHierarchies];
                        var hierDataReader = _memory.GetReaderAt(offHierarchies2);
                        if (hierDataReader != null)
                        {
                            for (int i = 0; i < numHierarchies; i++)
                            {
                                frame.Hierarchies[i] = new AnimHierarchy
                                {
                                    ChildChannelId = hierDataReader.ReadInt16(),
                                    ParentChannelId = hierDataReader.ReadInt16()
                                };
                            }
                        }
                    }
                }
            }

            frame.Hierarchies ??= [];
            frame.Channels ??= [];

            return frame;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads an AnimChannelMontreal structure.
    /// </summary>
    private AnimChannelMontreal? ReadChannel(int address)
    {
        if (address == 0) return null;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var channel = new AnimChannelMontreal { Address = address };

            // AnimChannelMontreal structure:
            // The first 4 bytes are actually the matrix pointer location
            // +0x00: uint32 isIdentity (if 1, no matrix data)
            // +0x04: byte objectIndex
            // +0x05: byte unk1
            // +0x06: short unk2
            // +0x08: short unk3
            // +0x0A: byte unkByte1
            // +0x0B: byte unkByte2
            // +0x0C: uint32 unkUint

            channel.OffMatrix = address; // Matrix is at the start of the channel
            channel.IsIdentity = reader.ReadUInt32();
            channel.ObjectIndex = reader.ReadSByte();
            channel.Unk1 = reader.ReadByte();
            channel.Unk2 = reader.ReadInt16();
            channel.Unk3 = reader.ReadInt16();
            channel.UnkByte1 = reader.ReadByte();
            channel.UnkByte2 = reader.ReadByte();
            channel.UnkUint = reader.ReadUInt32();

            // The first 4 bytes of a channel is actually a pointer to matrix data
            // If this value is 0 or 1, it's a special flag:
            // - 0: null/no transform
            // - 1: identity matrix
            // Otherwise, it's a virtual address pointing to compressed matrix data
            if (channel.IsIdentity != 1 && channel.IsIdentity != 0)
            {
                // The value is a virtual address - use it directly
                channel.Matrix = ReadCompressedMatrix((int)channel.IsIdentity);
            }
            else if (channel.IsIdentity == 1)
            {
                // Identity matrix
                channel.Matrix = new CompressedMatrix
                {
                    Position = Vector3.Zero,
                    Rotation = Quaternion.Identity,
                    Scale = Vector3.One
                };
            }

            return channel;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads a compressed matrix from memory.
    /// </summary>
    public CompressedMatrix? ReadCompressedMatrix(int address)
    {
        if (address == 0) return null;
        if (_matrixCache.TryGetValue(address, out var cached))
            return cached;

        var reader = _memory.GetReaderAt(address);
        if (reader == null) return null;

        try
        {
            var matrix = new CompressedMatrix();
            matrix.Type = reader.ReadUInt16();

            // The first byte & 0xF is the type:
            // 1: translation
            // 2: rotation only
            // 3: translation & rotation
            // 7: translation, rotation, uniform scale
            // 11: translation, rotation, per-axis scale
            // 15: translation, rotation, matrix scale

            int actualType = matrix.Type < 128 ? (matrix.Type & 0xF) : 128;

            // Translation
            if (actualType == 1 || actualType == 3 || actualType == 7 || actualType == 11 || actualType == 15)
            {
                float x = reader.ReadInt16() / 512f;
                float y = reader.ReadInt16() / 512f;
                float z = reader.ReadInt16() / 512f;
                matrix.Position = new Vector3(x, z, y); // Swap Y/Z for OpenSpace
            }

            // Rotation
            if (actualType == 2 || actualType == 3 || actualType == 7 || actualType == 11 || actualType == 15)
            {
                float w = reader.ReadInt16() / 32767f;
                float qx = reader.ReadInt16() / 32767f;
                float qy = reader.ReadInt16() / 32767f;
                float qz = reader.ReadInt16() / 32767f;

                // Convert quaternion with axis swap
                matrix.Rotation = new Quaternion(qx, qz, qy, -w);
            }
            else
            {
                matrix.Rotation = Quaternion.Identity;
            }

            // Scale
            if (actualType == 7)
            {
                // Uniform scale
                float s = reader.ReadInt16() / 256f;
                matrix.Scale = new Vector3(s, s, s);
            }
            else if (actualType == 11)
            {
                // Per-axis scale
                float sx = reader.ReadInt16() / 256f;
                float sy = reader.ReadInt16() / 256f;
                float sz = reader.ReadInt16() / 256f;
                matrix.Scale = new Vector3(sx, sz, sy); // Swap Y/Z
            }
            else if (actualType == 15)
            {
                // Matrix scale - read 6 values but just extract diagonal
                float m0 = reader.ReadInt16() / 256f;
                reader.ReadInt16(); // m1
                reader.ReadInt16(); // m2
                float m3 = reader.ReadInt16() / 256f;
                reader.ReadInt16(); // m4
                float m5 = reader.ReadInt16() / 256f;
                matrix.Scale = new Vector3(m0, m5, m3); // Swap Y/Z
            }
            else
            {
                matrix.Scale = Vector3.One;
            }

            _matrixCache[address] = matrix;
            return matrix;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Reads a standard 4x4 matrix from a BinaryReader.
    /// </summary>
    private static Matrix4x4 ReadMatrix4x4(BinaryReader reader)
    {
        var m = new Matrix4x4();
        m.M11 = reader.ReadSingle();
        m.M12 = reader.ReadSingle();
        m.M13 = reader.ReadSingle();
        m.M14 = reader.ReadSingle();
        m.M21 = reader.ReadSingle();
        m.M22 = reader.ReadSingle();
        m.M23 = reader.ReadSingle();
        m.M24 = reader.ReadSingle();
        m.M31 = reader.ReadSingle();
        m.M32 = reader.ReadSingle();
        m.M33 = reader.ReadSingle();
        m.M34 = reader.ReadSingle();
        m.M41 = reader.ReadSingle();
        m.M42 = reader.ReadSingle();
        m.M43 = reader.ReadSingle();
        m.M44 = reader.ReadSingle();
        return m;
    }
}
