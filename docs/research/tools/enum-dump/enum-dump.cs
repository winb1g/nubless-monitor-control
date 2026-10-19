// Prints every enum in a managed assembly as `Type.Member = value` (names are often NOT obfuscated even when code is).
// usage: dotnet enum-dump.dll <assembly.dll> [typeNameFilter]
using System;
using System.IO;
using System.Reflection.Metadata;
using System.Reflection.PortableExecutable;

var path = args[0];
var filter = args.Length > 1 ? args[1] : "";
using var fs = File.OpenRead(path);
using var pe = new PEReader(fs);
var md = pe.GetMetadataReader();

foreach (var th in md.TypeDefinitions)
{
    var t = md.GetTypeDefinition(th);
    if (t.BaseType.IsNil || t.BaseType.Kind != HandleKind.TypeReference) continue;
    var bt = md.GetTypeReference((TypeReferenceHandle)t.BaseType);
    if (md.GetString(bt.Name) != "Enum" || md.GetString(bt.Namespace) != "System") continue;

    var name = md.GetString(t.Name);
    if (filter != "" && !name.Contains(filter, StringComparison.OrdinalIgnoreCase)) continue;
    Console.WriteLine($"## {name}");
    foreach (var fh in t.GetFields())
    {
        var f = md.GetFieldDefinition(fh);
        var fname = md.GetString(f.Name);
        if (fname == "value__") continue;
        var ch = f.GetDefaultValue();
        if (ch.IsNil) continue;
        var c = md.GetConstant(ch);
        var r = md.GetBlobReader(c.Value);
        object v = c.TypeCode switch
        {
            ConstantTypeCode.Byte => r.ReadByte(),
            ConstantTypeCode.SByte => r.ReadSByte(),
            ConstantTypeCode.Int16 => r.ReadInt16(),
            ConstantTypeCode.UInt16 => r.ReadUInt16(),
            ConstantTypeCode.Int32 => r.ReadInt32(),
            ConstantTypeCode.UInt32 => r.ReadUInt32(),
            ConstantTypeCode.Int64 => r.ReadInt64(),
            ConstantTypeCode.UInt64 => r.ReadUInt64(),
            _ => "?",
        };
        Console.WriteLine($"{name}.{fname} = {v}");
    }
}
