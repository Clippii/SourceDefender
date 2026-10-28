using System;
using System.IO;
using System.Collections.Generic;

namespace SourceDefender {
    static class Program {
        static int Main(string[] args) {
            Console.InputEncoding=new System.Text.UTF8Encoding(false);
            Console.OutputEncoding=new System.Text.UTF8Encoding(false);
            if(args.Length>0&&args[0]=="--mcp")return McpHost.Run();
            if(args.Length>0&&args[0]=="--self-test") {
                string records=Path.Combine(Path.GetTempPath(),"SourceDefender-selftest-records-"+Guid.NewGuid().ToString("N"));
                Environment.SetEnvironmentVariable("SOURCEDEFENDER_DATA_DIR",records);
                try{return SelfTest.Run();}finally{if(Directory.Exists(records))Directory.Delete(records,true);}
            }
            var json=new JavaScriptSerializer();
            try {
                var req=json.Deserialize<Dictionary<string,object>>(Console.In.ReadToEnd());
                string command=(string)req["command"];object data;
                if(command=="snapshot")data=GameService.Snapshot();
                else if(command=="generate") {
                    var run=GameService.Start(Convert.ToInt32(req["scenario"]),(string)req["language"],Convert.ToInt32(req["level"]),
                        req.ContainsKey("seed")?(string)req["seed"]:null,(string)req["folder"],GameService.Human);
                    data=new {Run=GameService.PublicRun(run),Report=GameService.Brief(run)};
                } else {
                    var result=GameService.Act((string)req["id"],GameService.Human,command);
                    data=new {Run=GameService.PublicRun(result.Run),Report=result.Report};
                }
                Console.Write(json.Serialize(new {ok=true,data}));return 0;
            } catch(Exception e){Console.Write(json.Serialize(new {ok=false,error=e.Message}));return 1;}
        }
    }
}
