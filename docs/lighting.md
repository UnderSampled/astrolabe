# OpenSpace Engine Lighting System

This document describes the lighting system used in the OpenSpace Montreal engine (Rayman 2, Hype: The Time Quest, Tonic Trouble) based on the raymap reference implementation.

## Overview

The OpenSpace engine uses a **hybrid lighting system** combining:
- Dynamic per-vertex Gouraud shading
- Pre-baked vertex colors (radiosity)
- Sector-based light management
- Material lighting coefficients

The system was designed for 1999-era hardware (Dreamcast, PS1/PS2, PC) and avoids real-time shadow computation in favor of pre-baked data.

## Light Types

```csharp
public enum LightType {
    Unknown = 0,
    Parallel = 1,           // Directional light (sun, moon)
    Spherical = 2,          // Point light (torch, lamp)
    Hotspot = 3,            // Cone/spotlight
    Ambient = 4,            // Ambient fill light
    ParallelOtherType = 5,  // Directional with bounding box
    Fog = 6,                // Fog + background/sky color
    ParallelInASphere = 7,  // Directional within spherical bounds
    SphereOtherType = 8     // Point light ignoring persos
}
```

## LightInfo Structure

Each light contains the following data:

### Control Flags
| Field | Type | Description |
|-------|------|-------------|
| `turnedOn` | byte | Light on/off state |
| `castShadows` | byte | Shadow casting flag |
| `giroPhare` | byte | Rotating light flag |
| `pulse` | byte | Pulsing/animated light |
| `sendLightFlag` | byte | Light enabled (non-zero = on) |

### Spatial Parameters
| Field | Type | Description |
|-------|------|-------------|
| `type` | ushort | Light type enum |
| `near` | float | Near clipping distance |
| `far` | float | Far clipping/attenuation distance |
| `transMatrix` | Matrix | Position, rotation, scale |
| `interMinPos/MaxPos` | Vector3 | Interior bounding box |
| `exterMinPos/MaxPos` | Vector3 | Exterior bounding box |

### Color and Intensity
| Field | Type | Description |
|-------|------|-------------|
| `color` | Vector4 | RGBA light color |
| `background_color` | Vector4 | Background/fog color (type 6) |
| `shadowIntensity` | float | Shadow darkness multiplier |
| `attFactor3` | float | Attenuation factor |
| `intensityMin` | float | Minimum intensity |
| `intensityMax` | float | Maximum intensity |

### Animation Parameters
| Field | Type | Description |
|-------|------|-------------|
| `giroAngle` | float | Current rotation angle |
| `giroStep` | float | Rotation speed per frame |
| `pulseStep` | float | Pulse animation speed |
| `pulseMaxRange` | float | Maximum pulse range |

### Affect Flags

Lights can selectively affect different object types:

```csharp
[Flags]
public enum ObjectLightedFlag {
    None = 0,
    Environment = 1,    // Affects static geometry (IPOs)
    Perso = 2          // Affects characters/actors
}
```

Additional flags:
- `paintingLightFlag` - Affects painted/lightmapped surfaces
- `alphaLightFlag` - Affects transparency (0=color+alpha, 1=alpha only, 2=color only)

## Sector-Based Lighting

Lights are bound to **sectors** for spatial partitioning. Each sector maintains:

```csharp
public Reference<LightInfoArray> lights;  // Array of lights in this sector
public ushort num_lights;                  // Light count
```

When the camera enters a sector, its lights become active. This enables efficient culling of lights not affecting visible geometry.

## Vertex Lighting (Gouraud Shading)

The engine performs **per-vertex lighting calculations** using Gouraud shading. The shader supports three modes controlled by the `_Prelit` parameter:

| Mode | Value | Description |
|------|-------|-------------|
| Dynamic | 0 | Real-time lighting from dynamic lights |
| Pre-baked | 1 | Use vertex colors directly (baked lighting) |
| Hybrid | 2 | Combine pre-baked colors with dynamic lighting |

### Shader Implementation

```hlsl
if (_Prelit == 0) {
    // Calculate lighting from static light array
    for (int i = 0; i < _StaticLightCount; i++) {
        float3 lightDir = normalize(_StaticLightPos[i].xyz - worldPos);
        float diff = max(0.0, dot(normal, lightDir));
        color += _StaticLightCol[i].rgb * diff * attenuation;
    }
} else if (_Prelit == 1) {
    // Pre-baked vertex colors
    color = _DiffuseCoef * vertexColor;
} else if (_Prelit == 2) {
    // Hybrid: computed + vertex colors
    color = computedLight + _DiffuseCoef * vertexColor;
}
```

### Static Light Array

The shader supports up to 512 static lights per renderer:

```hlsl
float4 _StaticLightPos[512];    // Position (xyz) + type (w)
float4 _StaticLightDir[512];    // Direction for directional lights
float4 _StaticLightCol[512];    // Color (rgb) + alpha influence (w)
float4 _StaticLightParams[512]; // Near, far, flags
```

## Radiosity / Baked Vertex Colors

Pre-computed lighting is stored in **RadiosityLOD** structures:

```csharp
public class RadiosityLOD {
    public ColorISI[] colors;  // Per-vertex baked colors
}

public struct ColorISI {
    public short r, g, b, a;   // 16-bit signed color values

    public Color Color => new Color(
        Mathf.Clamp01(r / 256f),
        Mathf.Clamp01(g / 256f),
        Mathf.Clamp01(b / 256f),
        Mathf.Clamp01(a / 256f)
    );
}
```

Each `GeometricObject` can have associated radiosity data:

```csharp
public class GeometricObject {
    public RadiosityLOD radiosity;  // Baked lighting LOD
}
```

During mesh creation, radiosity colors are applied to vertices:

```csharp
if (geo.radiosity != null) {
    vertexColors[i] = geo.radiosity.colors[vertexIndex].Color;
}
```

## Shadows

### Shadow Casting

Lights have shadow casting properties:

```csharp
public byte castShadows;           // Does this light cast shadows?
public float shadowIntensity;      // Shadow darkness (0-1)
public uint createsShadowsOrNot;   // Shadow creation flag (R3/R2Revolution)
```

### Shadow Reception

Materials can opt-in to receiving shadows:

```csharp
public static uint property_receiveShadows = 2;  // Bit flag

bool receiveShadows = (material.properties & property_receiveShadows) != 0;
renderer.receiveShadows = receiveShadows;
```

### Shadow Geometry (Dinosaur Only)

Some game variants have dedicated shadow geometry objects:

```csharp
public class GeometricShadowObject {
    public GeometricObject data;  // Shadow mesh data
}
```

These are separate simplified meshes projected onto surfaces. Note: Currently disabled in raymap (`igo.Gao.SetActive(false)`) with comment "Shadows don't draw well right now".

### Shadow Implementation Notes

The engine does **not** use real-time shadow mapping. Shadows are achieved through:
1. Pre-baked vertex colors (ambient occlusion baked into radiosity)
2. Material flags controlling shadow reception
3. Dedicated shadow geometry in some variants
4. "Negative intensity" lights that subtract light (unsupported in additive renderers)

## Lightmaps (Largo Winch Only)

The Largo Winch game variant supports texture-based lightmaps:

```csharp
public Texture2D lightmap;
public Vector2[] lightmapUVs;
public int lightmap_index;
```

Lightmaps are applied as an additional texture layer with operation mode 50:

```hlsl
// Texture operation 50: Lightmap blend
return color_in + float4(lightmapColor.xyz, 0);
```

## Material Lighting Coefficients

Materials define how they respond to lighting:

```csharp
public class VisualMaterial {
    public Vector4 ambientCoef;   // Ambient light response
    public Vector4 diffuseCoef;   // Diffuse light response
    public Vector4 specularCoef;  // Specular response (limited use)
}
```

Platform variations exist:
- Some platforms default ambient to (0,0,0,1)
- Coefficient storage layout varies by platform

## Fog System

Type 6 lights (Fog) serve dual purpose:
1. Define fog parameters (near/far blend distances)
2. Provide background/sky color

```csharp
// Fog light properties
float fogBlendNear;     // Start of fog blend
float fogBlendFar;      // Full fog distance
Vector4 background_color; // Sky/background color
bool fogInfinite;       // Infinite fog flag
```

Runtime fog settings:

```csharp
RenderSettings.fog = true;
RenderSettings.fogMode = FogMode.Linear;
RenderSettings.fogStartDistance = light.near;
RenderSettings.fogEndDistance = light.far;
RenderSettings.fogColor = light.background_color;
```

## Global Lighting Controls

The `LightManager` provides global controls:

```csharp
public float luminosity;  // Global brightness (0-1)
public bool saturate;     // Color saturation toggle

// Shader globals
Shader.SetGlobalFloat("_Luminosity", luminosity);
Shader.SetGlobalFloat("_Saturate", saturate ? 1f : 0f);
Shader.SetGlobalFloat("_DisableLighting", disabled ? 1f : 0f);
Shader.SetGlobalFloat("_DisableFog", fogDisabled ? 1f : 0f);
```

## Reference Files

Key implementation files in raymap:
- `Assets/Scripts/OpenSpace/Visual/LightInfo.cs` - Light data structure
- `Assets/Scripts/OpenSpace/Visual/RadiosityLOD.cs` - Baked lighting
- `Assets/Scripts/Unity/LightManager.cs` - Runtime light management
- `Assets/Scripts/Unity/LightBehaviour.cs` - Unity light wrapper
- `Assets/Shaders/GouraudShared.cginc` - Vertex lighting shader

## glTF Export Considerations

When exporting to glTF:

| OpenSpace Type | glTF Equivalent |
|----------------|-----------------|
| Parallel | `KHR_lights_punctual` directional |
| Spherical | `KHR_lights_punctual` point |
| Hotspot | `KHR_lights_punctual` spot |
| Ambient | Scene ambient or baked into vertex colors |
| Fog | No direct equivalent; export as metadata |

Vertex colors (radiosity) should be exported as glTF vertex colors with `COLOR_0` attribute. Materials using pre-lit mode should have vertex colors enabled in the glTF material.

For shadows:
- Baked shadows in vertex colors export naturally
- Shadow reception flags have no glTF equivalent
- Consider baking shadow intensity into vertex color alpha
