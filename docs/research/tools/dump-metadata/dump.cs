using System;
using System.IO;
using System.Linq;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;

var path = args[0];
using var fs = File.OpenRead(path);
using var pe = new PEReader(fs);
var md = pe.GetMetadataReader();

Console.WriteLine("### USER STRINGS");
var h = MetadataTokens.UserStringHandle(0);
while (!h.IsNil)
{
    var s = md.GetUserString(h);
    if (s.Length > 0) Console.WriteLine($"[{MetadataTokens.GetHeapOffset(h):X}] {s.Replace("\r", "\\r").Replace("\n", "\\n")}");
    h = md.GetNextHandle(h);
}

Console.WriteLine("### TYPES/METHODS");
foreach (var th in md.TypeDefinitions)
{
    var t = md.GetTypeDefinition(th);
    var tn = md.GetString(t.Namespace) + "." + md.GetString(t.Name);
    foreach (var mh in t.GetMethods())
        Console.WriteLine($"{tn}::{md.GetString(md.GetMethodDefinition(mh).Name)}");
    foreach (var fh in t.GetFields())
        Console.WriteLine($"{tn}::[field]{md.GetString(md.GetFieldDefinition(fh).Name)}");
}
