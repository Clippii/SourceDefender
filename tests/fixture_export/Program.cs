using System.Text.Json;
using SourceDefender;
if(args.Length!=2) throw new ArgumentException("language and difficulty required");
var controls=ProjectTemplates.CatalogFor(args[0],int.Parse(args[1]),ProjectTemplates.Version);
Console.WriteLine(JsonSerializer.Serialize(controls.Select(c=>new {c.Id,c.Path,c.Safe})));
