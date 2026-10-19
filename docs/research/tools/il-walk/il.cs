using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;

var path = args[0];
var typeFilter = args[1];
var methFilter = args.Length > 2 ? args[2] : "";
if (methFilter == "*") methFilter = "";
if (typeFilter == "*") typeFilter = "";
using var fs = File.OpenRead(path);
using var pe = new PEReader(fs);
var md = pe.GetMetadataReader();

var one = new Dictionary<short, OpCode>();
foreach (var f in typeof(OpCodes).GetFields(BindingFlags.Public | BindingFlags.Static))
    if (f.GetValue(null) is OpCode oc) one[oc.Value] = oc;

string TypeName(EntityHandle h)
{
    switch (h.Kind)
    {
        case HandleKind.TypeDefinition: { var t = md.GetTypeDefinition((TypeDefinitionHandle)h); return md.GetString(t.Namespace) + "." + md.GetString(t.Name); }
        case HandleKind.TypeReference: { var t = md.GetTypeReference((TypeReferenceHandle)h); return md.GetString(t.Namespace) + "." + md.GetString(t.Name); }
        case HandleKind.TypeSpecification: return "<typespec>";
    }
    return "?";
}
string Resolve(int token)
{
    var h = MetadataTokens.EntityHandle(token);
    try
    {
        switch (h.Kind)
        {
            case HandleKind.MethodDefinition: { var m = md.GetMethodDefinition((MethodDefinitionHandle)h); return TypeName(m.GetDeclaringType()) + "::" + md.GetString(m.Name); }
            case HandleKind.MemberReference: { var m = md.GetMemberReference((MemberReferenceHandle)h); return TypeName(m.Parent) + "::" + md.GetString(m.Name); }
            case HandleKind.FieldDefinition: { var f = md.GetFieldDefinition((FieldDefinitionHandle)h); return TypeName(f.GetDeclaringType()) + "::" + md.GetString(f.Name) + " [field]"; }
            case HandleKind.TypeDefinition: case HandleKind.TypeReference: case HandleKind.TypeSpecification: return TypeName(h);
            case HandleKind.MethodSpecification: { var ms = md.GetMethodSpecification((MethodSpecificationHandle)h); return "spec:" + Resolve(MetadataTokens.GetToken(ms.Method)); }
        }
    }
    catch { }
    return $"tok:{token:X8}";
}

if (typeFilter.StartsWith("0x"))
{
    int tok = Convert.ToInt32(typeFilter.Substring(2), 16);
    var mh = (MethodDefinitionHandle)MetadataTokens.EntityHandle(tok);
    var m = md.GetMethodDefinition(mh);
    Console.WriteLine($"=== token {tok:X8} in {TypeName(m.GetDeclaringType())} sigblob={BitConverter.ToString(md.GetBlobBytes(m.Signature))}");
    var body = pe.GetMethodBody(m.RelativeVirtualAddress);
    Console.WriteLine("locals token: " + body.LocalSignature.IsNil + " maxstack " + body.MaxStack);
    var il = body.GetILBytes();
    int i = 0;
    while (i < il.Length)
    {
        short code = il[i++];
        if (code == 0xFE) code = (short)(0xFE00 | il[i++]);
        if (!one.TryGetValue(code, out var oc)) { Console.WriteLine($"  ?? {code:X}"); break; }
        string op = "";
        switch (oc.OperandType)
        {
            case OperandType.InlineNone: break;
            case OperandType.ShortInlineBrTarget: case OperandType.ShortInlineI: case OperandType.ShortInlineVar: op = ((sbyte)il[i]).ToString(); i += 1; break;
            case OperandType.InlineVar: i += 2; break;
            case OperandType.InlineI8: case OperandType.InlineR: op = BitConverter.ToInt64(il, i).ToString(); i += 8; break;
            case OperandType.InlineSwitch: { int n = BitConverter.ToInt32(il, i); i += 4 + 4 * n; break; }
            case OperandType.InlineMethod: case OperandType.InlineField: case OperandType.InlineType: case OperandType.InlineTok:
                op = Resolve(BitConverter.ToInt32(il, i)) + $" [{BitConverter.ToInt32(il, i):X8}]"; i += 4; break;
            case OperandType.InlineI: case OperandType.InlineBrTarget: case OperandType.ShortInlineR: op = BitConverter.ToInt32(il, i).ToString(); i += 4; break;
            default: i += 4; break;
        }
        Console.WriteLine($"  {oc.Name} {op}");
    }
    return;
}
foreach (var th in md.TypeDefinitions)
{
    var t = md.GetTypeDefinition(th);
    var tn = md.GetString(t.Namespace) + "." + md.GetString(t.Name);
    if (!tn.Contains(typeFilter)) continue;
    foreach (var mh in t.GetMethods())
    {
        var m = md.GetMethodDefinition(mh);
        var mn = md.GetString(m.Name);
        if (methFilter != "" && !mn.Contains(methFilter)) continue;
        Console.WriteLine($"=== {tn}::{mn}");
        if (m.RelativeVirtualAddress == 0) continue;
        var body = pe.GetMethodBody(m.RelativeVirtualAddress);
        var il = body.GetILBytes();
        int i = 0;
        while (i < il.Length)
        {
            short code = il[i++];
            if (code == 0xFE) code = (short)(0xFE00 | il[i++]);
            if (!one.TryGetValue(code, out var oc)) { Console.WriteLine($"  ?? {code:X}"); break; }
            string op = "";
            switch (oc.OperandType)
            {
                case OperandType.InlineNone: break;
                case OperandType.ShortInlineBrTarget: case OperandType.ShortInlineI: case OperandType.ShortInlineVar: op = il[i].ToString(); i += 1; break;
                case OperandType.InlineVar: i += 2; break;
                case OperandType.InlineI8: case OperandType.InlineR: i += 8; break;
                case OperandType.InlineSwitch: { int n = BitConverter.ToInt32(il, i); i += 4 + 4 * n; break; }
                case OperandType.InlineMethod: case OperandType.InlineField: case OperandType.InlineType: case OperandType.InlineTok:
                    op = Resolve(BitConverter.ToInt32(il, i)) + $" [{BitConverter.ToInt32(il, i):X8}]"; i += 4; break;
                case OperandType.InlineString:
                    { int tk = BitConverter.ToInt32(il, i); i += 4; op = "\"" + md.GetUserString(MetadataTokens.UserStringHandle(tk & 0xFFFFFF)) + "\""; break; }
                case OperandType.InlineI: case OperandType.InlineBrTarget: case OperandType.ShortInlineR: op = BitConverter.ToInt32(il, i).ToString(); i += 4; break;
                case OperandType.InlineSig: i += 4; break;
                default: i += 4; break;
            }
            Console.WriteLine($"  {oc.Name} {op}");
        }
    }
}
