using System;
using System.IO;
using System.Text.Json;

namespace Service {
    static class Program {
        static int Main(string[] args) {
            try {
                string root=Directory.GetCurrentDirectory();
                if(args.Length>0&&args[0]=="--test")return ExerciseTests.Run();
                var app=Bootstrap.Build(root);object result;
                switch(args.Length==0?"health":args[0]) {
                    case "health":result=app.Health();break;
                    case "list":result=app.List(Bootstrap.ExampleSession(),0,20);break;
                    case "export":result=app.Export(Bootstrap.ExampleSession());break;
                    case "reconcile":result=Workflows.Reconcile(app.Repository);break;
                    default:Console.Error.WriteLine("Usage: demo.exe health|list|export|reconcile|--test");return 1;
                }
                Console.WriteLine(JsonSerializer.Serialize(result,new JsonSerializerOptions {IncludeFields=true}));return 0;
            } catch(Exception error) {Console.Error.WriteLine(error.Message);return 1;}
        }
    }
}
