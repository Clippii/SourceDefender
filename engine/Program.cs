using System;
using System.IO;
using System.Linq;
using System.Collections.Generic;
namespace SourceDefender {
 static class Program {
  static JavaScriptSerializer json=new JavaScriptSerializer();
  static int Main(string[] args) {
   Console.InputEncoding=new System.Text.UTF8Encoding(false);
   Console.OutputEncoding=new System.Text.UTF8Encoding(false);
   if(args.Length>0&&args[0]=="--self-test") {
    string records=Path.Combine(Path.GetTempPath(),"SourceDefender-selftest-records-"+Guid.NewGuid().ToString("N"));
    Environment.SetEnvironmentVariable("SOURCEDEFENDER_DATA_DIR",records);
    try {return SelfTest.Run();}finally {if(Directory.Exists(records))Directory.Delete(records,true);}
   }
   try {
    var req=json.Deserialize<Dictionary<string,object>>(Console.In.ReadToEnd());
    string cmd=(string)req["command"]; object data;
    if(cmd=="snapshot") data=new {Runs=Store.Load(),Scenarios=Scenario.All,Difficulties=Difficulty.All,Languages=Catalog.Languages};
    else if(cmd=="generate") {
     int level=Convert.ToInt32(req["level"]),scenario=Convert.ToInt32(req["scenario"]);
     string language=(string)req["language"],folder=Path.GetFullPath((string)req["folder"]);
     if(level<0||level>=4||scenario<0||scenario>=Scenario.All.Length||!Catalog.Languages.Contains(language))throw new Exception("Invalid incident settings.");
     if(Store.Load().Any(x=>x.Status=="Active"))throw new Exception("End the active incident before generating another.");
     var generated=Engine.Generate(folder,level,language,Scenario.All[scenario],(string)req["seed"]);
     data=new {Run=generated,Report=generated.Events.Last()+"\n\nRead README.md, docs/architecture.md, and HANDBOOK.md. Trace the application and policy services, investigate in your editor, save repairs, then validate."};
    } else {
     string id=(string)req["id"];var r=Store.Load().FirstOrDefault(x=>x.Id==id);
     if(r==null)throw new Exception("Incident no longer exists.");
     if(r.Status!="Active")throw new Exception("This incident has already ended.");
     string report="";
     if(cmd=="end") {r.Status="Abandoned";r.FinishedUtc=DateTime.UtcNow.ToString("o");report="Run ended. Your repository remains on disk.";}
     else if(cmd=="validate") {
      var failed=Engine.Scan(r);r.Attempts++;
      if(failed.Count==0) {r.Status="Contained";r.FinishedUtc=DateTime.UtcNow.ToString("o");report="INCIDENT CONTAINED\n\nAll 16 behavior controls passed. Your repository stays on disk. Review its Git diff for a debrief.";}
      else {
       r.PenaltySeconds+=Difficulty.All[r.Level].Penalty;
       report="VALIDATION FAILED / "+failed.Count+" controls unresolved\n+"+Difficulty.All[r.Level].Penalty+" seconds\n\n";
       if(r.Level==0)foreach(var c in failed)report+=c.Title+"\n"+c.Path+"\n\n";
       else if(r.Level==1)foreach(var c in failed)report+="["+c.Category+"] "+c.Title+"\n";
       else if(r.Level==2)foreach(var g in failed.GroupBy(c=>c.Category))report+=g.Key+": "+g.Count()+"\n";
       else report+="Detailed findings are disabled at Zero trust. Trace the changes.\n";
       report+="\nMissing files and regressions also fail validation. Save changes before scanning again.";
      }
      r.Events.Add(report);
     } else if(cmd=="hint") {
      var failed=Engine.Scan(r);var c=failed.FirstOrDefault(x=>!r.Revealed.Contains(x.Id));
      if(c==null)report=failed.Count==0?"All controls are clean. Validate to finish.":"Intel has already been provided for every remaining control.";
      else {r.Hints++;r.PenaltySeconds+=Difficulty.All[r.Level].HintCost;r.Revealed.Add(c.Id);report="INTEL / +"+Difficulty.All[r.Level].HintCost+" seconds\n\n"+c.Title+"\n"+c.Path+"\n\n"+c.Hint;r.Events.Add(report);}
     } else throw new Exception("Unknown command.");
     Store.Save(r);data=new {Run=r,Report=report};
    }
    Console.Write(json.Serialize(new {ok=true,data=data}));return 0;
   } catch(Exception e){Console.Write(json.Serialize(new {ok=false,error=e.Message}));return 1;}
  }
 }
}
