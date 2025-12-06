namespace Astrolabe.Core.FileFormats.AI;

/// <summary>
/// Simple test class to verify S-expression conversion works correctly.
/// Run with: dotnet run --project src/Astrolabe.Core -- test-sexpr
/// </summary>
public static class SExpressionConverterTests
{
    /// <summary>
    /// Creates a test script matching the example from the documentation:
    /// (if (cond-equal (dsgvar 0) (constant 5))
    ///   (then (proc-show-text (text-ref 42))))
    /// </summary>
    public static Script CreateTestScript()
    {
        var script = new Script { Offset = 0 };

        // [param=0,  indent=1, type=KeyWord_If]        -> If
        script.Nodes.Add(new ScriptNode { Offset = 0, Param = 0, Indent = 1, Type = 0, NodeType = NodeType.KeyWord });

        // [param=3,  indent=2, type=Condition]         -> Cond_Equal (index 3 in Hype)
        script.Nodes.Add(new ScriptNode { Offset = 8, Param = 3, Indent = 2, Type = 1, NodeType = NodeType.Condition });

        // [param=0,  indent=3, type=DsgVarRef]         -> dsgvar 0
        script.Nodes.Add(new ScriptNode { Offset = 16, Param = 0, Indent = 3, Type = 11, NodeType = NodeType.DsgVarRef });

        // [param=5,  indent=3, type=Constant]          -> 5
        script.Nodes.Add(new ScriptNode { Offset = 24, Param = 5, Indent = 3, Type = 12, NodeType = NodeType.Constant });

        // [param=1,  indent=2, type=KeyWord_Then]      -> Then
        script.Nodes.Add(new ScriptNode { Offset = 32, Param = 1, Indent = 2, Type = 0, NodeType = NodeType.KeyWord });

        // [param=127, indent=3, type=Procedure]        -> Proc_DisplayString (closest to ShowText)
        script.Nodes.Add(new ScriptNode { Offset = 40, Param = 127, Indent = 3, Type = 4, NodeType = NodeType.Procedure });

        // [param=42, indent=4, type=TextRef]           -> text-ref 42
        script.Nodes.Add(new ScriptNode { Offset = 48, Param = 42, Indent = 4, Type = 30, NodeType = NodeType.TextRef });

        // [param=0,  indent=0, type=End]               -> end marker
        script.Nodes.Add(new ScriptNode { Offset = 56, Param = 0, Indent = 0, Type = 0, NodeType = NodeType.Unknown });

        return script;
    }

    /// <summary>
    /// Runs a basic test of the S-expression converter.
    /// </summary>
    public static void RunTest()
    {
        var script = CreateTestScript();
        var converter = new SExpressionConverter(AITypes.Hype);

        string sexpr = converter.Convert(script);

        Console.WriteLine("=== S-Expression Output ===");
        Console.WriteLine(sexpr);
        Console.WriteLine();
        Console.WriteLine("=== Expected Structure ===");
        Console.WriteLine(@"(if
  (cond-equal (dsgvar 0) (const 5))
  (then
    (proc-display-string
      (text-ref 42))))");
    }

    /// <summary>
    /// Creates a more complex test script with operators and multiple statements.
    /// </summary>
    public static Script CreateComplexTestScript()
    {
        var script = new Script { Offset = 0 };
        int offset = 0;

        // If condition
        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 0, Indent = 1, Type = 0, NodeType = NodeType.KeyWord }); // If
        offset += 8;

        // condition: dsgvar_0 > 10
        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 6, Indent = 2, Type = 1, NodeType = NodeType.Condition }); // Cond_Greater
        offset += 8;

        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 0, Indent = 3, Type = 11, NodeType = NodeType.DsgVarRef }); // dsgvar 0
        offset += 8;

        // 10 as constant
        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 10, Indent = 3, Type = 12, NodeType = NodeType.Constant }); // 10
        offset += 8;

        // Then block
        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 1, Indent = 2, Type = 0, NodeType = NodeType.KeyWord }); // Then
        offset += 8;

        // Assignment: dsgvar_1 = dsgvar_0 + 5
        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 11, Indent = 3, Type = 2, NodeType = NodeType.Operator }); // Affect (set!)
        offset += 8;

        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 1, Indent = 4, Type = 11, NodeType = NodeType.DsgVarRef }); // dsgvar 1
        offset += 8;

        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 0, Indent = 4, Type = 2, NodeType = NodeType.Operator }); // Plus
        offset += 8;

        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 0, Indent = 5, Type = 11, NodeType = NodeType.DsgVarRef }); // dsgvar 0
        offset += 8;

        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 5, Indent = 5, Type = 12, NodeType = NodeType.Constant }); // 5
        offset += 8;

        // Else block
        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 2, Indent = 2, Type = 0, NodeType = NodeType.KeyWord }); // Else
        offset += 8;

        // Call a procedure
        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 3, Indent = 3, Type = 4, NodeType = NodeType.Procedure }); // Proc_ActivateObject
        offset += 8;

        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 4, Indent = 4, Type = 0, NodeType = NodeType.KeyWord }); // Me
        offset += 8;

        // End marker
        script.Nodes.Add(new ScriptNode { Offset = offset, Param = 0, Indent = 0, Type = 0, NodeType = NodeType.Unknown });

        return script;
    }

    /// <summary>
    /// Runs tests on both simple and complex scripts.
    /// </summary>
    public static void RunAllTests()
    {
        Console.WriteLine("=== Simple Script Test ===\n");
        RunTest();

        Console.WriteLine("\n\n=== Complex Script Test ===\n");

        var complexScript = CreateComplexTestScript();
        var converter = new SExpressionConverter(AITypes.Hype);

        string sexpr = converter.Convert(complexScript);
        Console.WriteLine(sexpr);
    }
}
