using Astrolabe.Core.Hub;
using Xunit;

namespace Astrolabe.Core.Tests;

public sealed class HubFragmentResolverTests
{
    [Theory]
    [InlineData("types/raw/foo.bin", "types/raw/foo.bin", 0)]
    [InlineData("types/raw/foo.bin#byteOffset=4", "types/raw/foo.bin", 4)]
    [InlineData("types/raw/foo.bin#byteOffset=0", "types/raw/foo.bin", 0)]
    public void TrySplitUri_ParsesValidFragments(string uri, string expectedPath, int expectedOffset)
    {
        Assert.True(HubFragmentResolver.TrySplitUri(uri, out var path, out var offset));
        Assert.Equal(expectedPath, path);
        Assert.Equal(expectedOffset, offset);
    }

    [Theory]
    [InlineData("types/raw/foo.bin#byteOffset=abc")]
    [InlineData("types/raw/foo.bin#foo=1")]
    [InlineData("types/raw/foo.bin#byteOffset=")]
    public void TrySplitUri_RejectsInvalidFragments(string uri)
    {
        Assert.False(HubFragmentResolver.TrySplitUri(uri, out _, out _));
    }

    [Fact]
    public void SplitUri_ThrowsOnInvalidFragment()
    {
        Assert.Throws<InvalidDataException>(() => HubFragmentResolver.SplitUri("types/raw/foo.bin#byteOffset=abc"));
    }
}