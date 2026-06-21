using Astrolabe.Core.Rete;

namespace Astrolabe.Core.Hub;

internal static class HubReferenceMaterializer
{
    public static void Materialize(object record, ReferenceAddressResolver resolver, string packageRoot)
    {
        foreach (var property in record.GetType().GetProperties())
        {
            if (property.PropertyType != typeof(HubReference) ||
                property.GetValue(record) is not HubReference reference ||
                reference.IsNull)
            {
                continue;
            }

            if (!string.IsNullOrWhiteSpace(reference.Uri))
            {
                reference.ResolvedAddress = resolver.ResolveAddress(packageRoot, reference.Uri);
            }
        }
    }

}