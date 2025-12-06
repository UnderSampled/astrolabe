using System.Globalization;
using System.Text;

namespace Astrolabe.Core.FileFormats.AI;

/// <summary>
/// Converts OpenSpace AI scripts to S-expression format.
///
/// The script node array uses indent levels to encode tree structure:
/// - A node at indent N+1 is a child of the previous node at indent N
/// - This makes the format a serialized S-expression tree
/// </summary>
public class SExpressionConverter
{
    private readonly AITypes _aiTypes;
    private readonly SExpressionOptions _options;

    public SExpressionConverter(AITypes aiTypes, SExpressionOptions? options = null)
    {
        _aiTypes = aiTypes;
        _options = options ?? new SExpressionOptions();
    }

    /// <summary>
    /// Converts a script to S-expression string.
    /// </summary>
    public string Convert(Script script)
    {
        if (script.Nodes.Count == 0)
            return "()";

        // Build tree from flat node list
        var root = BuildTree(script.Nodes);

        // Convert tree to S-expression
        var sb = new StringBuilder();
        WriteNode(root, sb, 0);
        return sb.ToString().TrimEnd();
    }

    /// <summary>
    /// Builds a tree structure from the flat list of nodes.
    /// </summary>
    private TreeNode BuildTree(List<ScriptNode> nodes)
    {
        // Create a virtual root node
        var root = new TreeNode(null, -1);
        var nodeWrappers = nodes.Select((n, i) => new TreeNode(n, i)).ToList();

        // Build tree using indent levels
        for (int i = 0; i < nodeWrappers.Count; i++)
        {
            var current = nodeWrappers[i];
            if (current.Node == null || current.Node.Indent == 0)
                continue; // Skip end marker

            // Find parent: look backwards for first node with indent = current.indent - 1
            TreeNode parent = root;
            for (int j = i - 1; j >= 0; j--)
            {
                var candidate = nodeWrappers[j];
                if (candidate.Node != null && candidate.Node.Indent == current.Node.Indent - 1)
                {
                    parent = candidate;
                    break;
                }
            }

            parent.Children.Add(current);
        }

        return root;
    }

    /// <summary>
    /// Writes a tree node as S-expression.
    /// </summary>
    private void WriteNode(TreeNode treeNode, StringBuilder sb, int depth)
    {
        // Root node (virtual) - just write children
        if (treeNode.Node == null)
        {
            foreach (var child in treeNode.Children)
            {
                WriteNode(child, sb, depth);
                sb.AppendLine();
            }
            return;
        }

        var node = treeNode.Node;

        // End marker
        if (node.Indent == 0)
            return;

        string indent = _options.IndentString != null
            ? new string(' ', depth * _options.IndentSize)
            : "";

        sb.Append(indent);

        // Format based on node type
        string symbol = GetSymbol(node);
        string? value = GetValue(node);

        if (treeNode.Children.Count == 0)
        {
            // Leaf node
            if (value != null)
                sb.Append($"({symbol} {value})");
            else
                sb.Append(symbol);
        }
        else
        {
            // Node with children
            sb.Append($"({symbol}");

            if (value != null)
                sb.Append($" {value}");

            bool singleLineChildren = treeNode.Children.Count <= 2 &&
                                       treeNode.Children.All(c => c.Children.Count == 0);

            if (singleLineChildren && !_options.AlwaysMultiline)
            {
                // Write children on same line
                foreach (var child in treeNode.Children)
                {
                    sb.Append(' ');
                    WriteNodeInline(child, sb);
                }
                sb.Append(')');
            }
            else
            {
                // Write children on new lines
                sb.AppendLine();
                foreach (var child in treeNode.Children)
                {
                    WriteNode(child, sb, depth + 1);
                    sb.AppendLine();
                }
                sb.Append(indent);
                sb.Append(')');
            }
        }
    }

    /// <summary>
    /// Writes a node inline (no newlines).
    /// </summary>
    private void WriteNodeInline(TreeNode treeNode, StringBuilder sb)
    {
        if (treeNode.Node == null || treeNode.Node.Indent == 0)
            return;

        var node = treeNode.Node;
        string symbol = GetSymbol(node);
        string? value = GetValue(node);

        if (treeNode.Children.Count == 0)
        {
            if (value != null)
                sb.Append($"({symbol} {value})");
            else
                sb.Append(symbol);
        }
        else
        {
            sb.Append($"({symbol}");
            if (value != null)
                sb.Append($" {value}");

            foreach (var child in treeNode.Children)
            {
                sb.Append(' ');
                WriteNodeInline(child, sb);
            }
            sb.Append(')');
        }
    }

    /// <summary>
    /// Gets the S-expression symbol for a node.
    /// </summary>
    private string GetSymbol(ScriptNode node)
    {
        return node.NodeType switch
        {
            NodeType.KeyWord => ToKebabCase(_aiTypes.GetKeyword(node.Param)),
            NodeType.Condition => ToKebabCase(_aiTypes.GetCondition(node.Param)),
            NodeType.Operator => GetOperatorSymbol(node.Param),
            NodeType.Function => ToKebabCase(_aiTypes.GetFunction(node.Param)),
            NodeType.Procedure => ToKebabCase(_aiTypes.GetProcedure(node.Param)),
            NodeType.MetaAction => ToKebabCase(_aiTypes.GetMetaAction(node.Param)),
            NodeType.Field => ToKebabCase(_aiTypes.GetField(node.Param)),
            NodeType.BeginMacro => "begin-macro",
            NodeType.EndMacro => "end-macro",
            NodeType.DsgVarRef => "dsgvar",
            NodeType.DsgVar => "dsgvar",
            NodeType.Constant => "const",
            NodeType.Real => "real",
            NodeType.Button => "button",
            NodeType.ConstantVector => "const-vector",
            NodeType.Vector => "vector",
            NodeType.Mask => "mask",
            NodeType.ModuleRef => "module-ref",
            NodeType.Module => "module",
            NodeType.DsgVarId => "dsgvar-id",
            NodeType.String => "string",
            NodeType.LipsSynchroRef => "lips-synchro-ref",
            NodeType.FamilyRef => "family-ref",
            NodeType.Way => "way",
            NodeType.PersoRef => "perso-ref",
            NodeType.ActionRef => "action-ref",
            NodeType.EnvironmentRef => "env-ref",
            NodeType.SuperObjectRef => "superobject-ref",
            NodeType.SurfaceRef => "surface-ref",
            NodeType.WayPointRef => "waypoint-ref",
            NodeType.TextRef => "text-ref",
            NodeType.FontRef => "font-ref",
            NodeType.ComportRef => "comport-ref",
            NodeType.SoundEventRef => "sound-event-ref",
            NodeType.ObjectTableRef => "object-table-ref",
            NodeType.GameMaterialRef => "game-material-ref",
            NodeType.ParticleGenerator => "particle-generator",
            NodeType.Color => "color",
            NodeType.ModelRef => "model-ref",
            NodeType.Caps => "caps",
            NodeType.GraphRef => "graph-ref",
            NodeType.CustomBits => "custom-bits",
            NodeType.Null => "null",
            NodeType.SubRoutine => "subroutine",
            _ => $"unknown-{node.Type}"
        };
    }

    /// <summary>
    /// Gets the operator symbol, using mathematical notation where appropriate.
    /// </summary>
    private string GetOperatorSymbol(uint param)
    {
        string op = _aiTypes.GetOperator(param);
        return op switch
        {
            "Operator_Plus" => "+",
            "Operator_Minus" => "-",
            "Operator_Mul" => "*",
            "Operator_Div" => "/",
            "Operator_UnaryMinus" => "neg",
            "Operator_PlusAffect" => "+=",
            "Operator_MinusAffect" => "-=",
            "Operator_MulAffect" => "*=",
            "Operator_DivAffect" => "/=",
            "Operator_PlusPlusAffect" => "++",
            "Operator_MinusMinusAffect" => "--",
            "Operator_Affect" => "set!",
            "Operator_Dot" => ".",
            ".X" => ".x",
            ".Y" => ".y",
            ".Z" => ".z",
            "Operator_VectorPlusVector" => "vec+",
            "Operator_VectorMinusVector" => "vec-",
            "Operator_VectorMulScalar" => "vec*",
            "Operator_VectorDivScalar" => "vec/",
            "Operator_VectorUnaryMinus" => "vec-neg",
            ".X:=" => "set-x!",
            ".Y:=" => "set-y!",
            ".Z:=" => "set-z!",
            "Operator_Ultra" => "ultra",
            "Operator_ModelCast" => "model-cast",
            "Operator_Array" => "array-ref",
            "Operator_AffectArray" => "array-set!",
            _ => ToKebabCase(op)
        };
    }

    /// <summary>
    /// Gets the value representation for a node (if any).
    /// </summary>
    private string? GetValue(ScriptNode node)
    {
        return node.NodeType switch
        {
            NodeType.DsgVarRef => node.Param.ToString(),
            NodeType.DsgVar => node.Param.ToString(),
            NodeType.DsgVarId => node.Param.ToString(),
            NodeType.Constant => BitConverter.ToInt32(BitConverter.GetBytes(node.Param), 0).ToString(),
            NodeType.Real => FormatFloat(BitConverter.ToSingle(BitConverter.GetBytes(node.Param), 0)),
            NodeType.TextRef => node.Param.ToString(),
            NodeType.SoundEventRef => $"0x{node.Param:X8}",
            NodeType.Mask => $"0x{(short)node.Param:X4}",
            NodeType.ModuleRef => node.Param.ToString(),
            NodeType.Module => node.Param.ToString(),
            NodeType.CustomBits => $"0x{node.Param:X8}",
            NodeType.Caps => $"0x{node.Param:X8}",
            NodeType.Color => $"0x{node.Param:X8}",
            NodeType.ParticleGenerator => $"0x{node.Param:X8}",
            // Pointer types - show raw address for now
            NodeType.PersoRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.WayPointRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.ComportRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.ActionRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.SuperObjectRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.ObjectTableRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.GameMaterialRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.FamilyRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.GraphRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.ModelRef => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.SubRoutine => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            NodeType.String => _options.ShowPointers ? $"@0x{node.Param:X8}" : null,
            _ => null
        };
    }

    /// <summary>
    /// Converts a name to kebab-case for S-expression symbols.
    /// </summary>
    private static string ToKebabCase(string name)
    {
        var sb = new StringBuilder();

        for (int i = 0; i < name.Length; i++)
        {
            char c = name[i];

            if (c == '_')
            {
                sb.Append('-');
            }
            else if (char.IsUpper(c))
            {
                if (i > 0 && sb.Length > 0 && sb[sb.Length - 1] != '-')
                {
                    // Don't add dash if previous char was also uppercase (acronym)
                    if (!char.IsUpper(name[i - 1]))
                        sb.Append('-');
                }
                sb.Append(char.ToLowerInvariant(c));
            }
            else
            {
                sb.Append(c);
            }
        }

        return sb.ToString();
    }

    /// <summary>
    /// Formats a float for S-expression output.
    /// </summary>
    private static string FormatFloat(float value)
    {
        return value.ToString("G", CultureInfo.InvariantCulture);
    }

    /// <summary>
    /// Tree node for building the parse tree.
    /// </summary>
    private class TreeNode
    {
        public ScriptNode? Node { get; }
        public int Index { get; }
        public List<TreeNode> Children { get; } = new();

        public TreeNode(ScriptNode? node, int index)
        {
            Node = node;
            Index = index;
        }
    }
}

/// <summary>
/// Options for S-expression output formatting.
/// </summary>
public class SExpressionOptions
{
    /// <summary>Whether to show pointer addresses for reference types.</summary>
    public bool ShowPointers { get; set; } = true;

    /// <summary>String to use for indentation (null for no indentation).</summary>
    public string? IndentString { get; set; } = "  ";

    /// <summary>Number of spaces per indent level.</summary>
    public int IndentSize { get; set; } = 2;

    /// <summary>Always use multiline format even for simple expressions.</summary>
    public bool AlwaysMultiline { get; set; } = false;
}
