using System;
using System.IO;
using System.Linq;
using System.Text;
using System.Collections.Generic;
using System.Diagnostics;
namespace SourceDefender {
 public static class Engine {
  public static Run Generate(string parent,int level,string language,Scenario scenario,string seed,string mode="human") {
   var r=new Run{Id=Guid.NewGuid().ToString("N"),Level=level,Language=language,Scenario=scenario.Name,Seed=string.IsNullOrWhiteSpace(seed)?Guid.NewGuid().ToString("N").Substring(0,8):seed,StartedUtc=DateTime.UtcNow.ToString("o"),Status="Active",TemplateVersion=ProjectTemplates.Version,Mode=mode,ToolCalls=mode=="agent_practice"?1:0};
   r.Folder=Path.Combine(parent,scenario.Slug+"-"+r.Id.Substring(0,8));
   Directory.CreateDirectory(r.Folder);
   try {
    var catalog=ProjectTemplates.CatalogFor(language,level,r.TemplateVersion); var shuffled=new List<Challenge>(catalog);
    uint hash=2166136261; foreach(char x in r.Seed) {hash^=x;hash*=16777619;} var random=new Random(unchecked((int)hash));
    for(int i=shuffled.Count-1;i>0;i--) {int j=random.Next(i+1);var t=shuffled[i];shuffled[i]=shuffled[j];shuffled[j]=t;}
    r.Broken=shuffled.Take(Difficulty.All[level].Bugs).Select(c=>c.Id).ToList();
    var support=ProjectTemplates.Files(language,level,scenario);
    if(level>=2 && support.ContainsKey("docs/operations.md")) {
     support["docs/operations.md"]=support["docs/operations.md"].Replace("\r\n","\n")
      .Replace("- Git history contains the trusted baseline and subsequent maintenance commits.","- Prior Git history and the trusted baseline are unavailable at this difficulty.")
      .Replace("Did a maintenance commit change review ownership or conceal source files?","Did repository configuration change review ownership or conceal source files?")
      .Replace("Comparing against or restoring the trusted baseline is valid containment. Save\nyour files before validation.","Use the behavior contract and the source itself as evidence. Save\nyour files before validation.");
    }
    foreach(var file in support)Write(r.Folder,file.Key,file.Value);
    r.RequiredFiles=support.Keys.Where(IsSource).ToList();
    foreach(var c in catalog) Write(r.Folder,c.Path,level>=2&&r.Broken.Contains(c.Id)?c.Unsafe:c.Safe);
    Write(r.Folder,"README.md","# "+scenario.Name+"\n\n"+scenario.Mission+"\n\nThis repository contains a multi-module local service: request routing, authentication, storage, review workflows, invoices, exports, telemetry, update plans, plugins, and background jobs. Domain data and record kinds are in config/scenario.json.\n\n## Incident\n"+(level>=2?"A trusted automation account was compromised. Find and remove attacker changes while preserving legitimate behavior. No prior Git history or trusted baseline is available; investigate the current files and behavior contract.":"A trusted automation account was compromised after the baseline commit. Find and remove attacker changes while preserving legitimate behavior. Use git log, git show and git diff to investigate. The baseline is available for forensic comparison. Restoring the baseline is a valid containment strategy.")+"\n\nStart with docs/architecture.md, docs/api.md, docs/data-dictionary.md, and HANDBOOK.md. Trace requests through the application, repository, and policy services. Regression tests describe expected business behavior. The game checks all 16 known controls and detects missing supporting source or test files.\n\nReview source and automation before running local commands. The game itself never executes generated code. Message delivery, payment, plugin execution, and remote updates are represented as in-memory plans; the service makes no network requests.\n\n## Local commands after review\n"+Commands(language)+"\n\nLanguage: "+language+"\nSeed: "+r.Seed+"\nTemplate version: "+r.TemplateVersion+"\n");
    var handbook=new StringBuilder("# Trusted behavior contract\n\n");
    foreach(var c in catalog) handbook.AppendLine("- "+c.Policy);
    handbook.AppendLine("\nSource repairs must retain each resolve_* API, its service entry point, and its documented literal value. Supporting source and tests must remain present. Configuration repairs must match the trusted behavior contract; whitespace is ignored. The bounded static game checks do not prove arbitrary code safe. After source review, the project's own tests exercise repository behavior and business trust boundaries.");
    Write(r.Folder,"HANDBOOK.md",handbook.ToString());
    bool git=false;
    if(level>=2)CreateEmptyGitBoundary(r.Folder);
    else git=Git(r.Folder,"init -q")&&Git(r.Folder,"-c user.name=SourceDefender -c user.email=training@example.invalid add .")&&Git(r.Folder,"-c user.name=SourceDefender -c user.email=training@example.invalid commit -qm \"Trusted service baseline\"");
    int batch=0;
    foreach(var c in shuffled.Take(Difficulty.All[level].Bugs)) {
     Write(r.Folder,c.Path,c.Unsafe); batch++;
     if(git&&(level>=2||batch==r.Broken.Count)) {
      Git(r.Folder,"add ."); Git(r.Folder,"-c user.name=build-helper -c user.email=bot@example.invalid commit -qm \""+new[]{"Align compatibility adapters","Refresh integration defaults","Update deployment metadata","Maintenance cleanup"}[batch%4]+"\"");
     }
    }
    int fileCount=Directory.GetFiles(r.Folder,"*",SearchOption.AllDirectories).Count(p=>!p.Contains(Path.DirectorySeparatorChar+".git"+Path.DirectorySeparatorChar));
    int codeLines=Directory.GetFiles(r.Folder,"*",SearchOption.AllDirectories).Where(IsSource).Sum(p=>File.ReadAllLines(p).Length);
    r.Events.Add("Repository generated · "+fileCount+" files · "+codeLines+" source lines · "+r.Broken.Count+" attacker changes · "+(level>=2?"Git history unavailable; investigate current files":git?"Git evidence available":"Git unavailable; file review available"));
    Store.Save(r); return r;
   } catch { r.Status="Generation failed";r.FinishedUtc=DateTime.UtcNow.ToString("o");Store.Save(r);throw; }
  }
  // An empty repository prevents Git from walking upward into a parent checkout.
  // It contains no commits, objects, hooks, reflogs, or trusted baseline.
  static void CreateEmptyGitBoundary(string folder) {
   string dir=Path.Combine(folder,".git");Directory.CreateDirectory(Path.Combine(dir,"objects"));Directory.CreateDirectory(Path.Combine(dir,"refs"));
   File.WriteAllText(Path.Combine(dir,"HEAD"),"ref: refs/heads/main\n",new UTF8Encoding(false));
   File.WriteAllText(Path.Combine(dir,"config"),"[core]\n\trepositoryformatversion = 0\n\tbare = false\n",new UTF8Encoding(false));
  }
  static void Write(string folder,string path,string text) { string p=Path.Combine(folder,path.Replace('/',Path.DirectorySeparatorChar));Directory.CreateDirectory(Path.GetDirectoryName(p));File.WriteAllText(p,text,new UTF8Encoding(false)); }
  public static bool IsSource(string path) {return new[]{".py",".js",".go",".cs"}.Contains(Path.GetExtension(path));}
  static string Commands(string language) {
   switch(language) {
    case "Python":return "Python 3.10+; standard library only.\n- python -m src.cli health\n- python -m src.cli reconcile\n- python -m unittest discover -s tests -v";
    case "JavaScript":return "Node.js 18+; no service dependency install needed.\n- node src/cli.js health\n- node src/cli.js reconcile\n- node --test tests/*.test.js";
    case "Go":return "Go 1.20+; standard library only.\n- go run ./cmd/demo health\n- go run ./cmd/demo reconcile\n- go test ./...";
    default:return ".NET 10 SDK; works on Windows, Linux, and macOS.\n- dotnet run --project Service.csproj -- health\n- dotnet run --project Service.csproj -- reconcile\n- dotnet run --project Service.csproj -- --test";
   }
  }
  public static List<Challenge> Scan(Run r) {
   var failed=new List<Challenge>();
   foreach(var c in ProjectTemplates.CatalogFor(r.Language,r.Level,r.TemplateVersion)) {
    string p=Path.Combine(r.Folder,c.Path.Replace('/',Path.DirectorySeparatorChar));
    try { var info=new FileInfo(p); if(!info.Exists||info.Length>1024*1024||(info.Attributes&FileAttributes.ReparsePoint)!=0||!c.Check(File.ReadAllText(p))) failed.Add(c); } catch {failed.Add(c);}
   }
   foreach(string relative in r.RequiredFiles??new List<string>()) {
    string path=Path.Combine(r.Folder,relative.Replace('/',Path.DirectorySeparatorChar));
    bool present=false;
    try {var info=new FileInfo(path);present=info.Exists&&info.Length>=20&&info.Length<=1024*1024&&(info.Attributes&FileAttributes.ReparsePoint)==0&&File.ReadAllText(path).Trim().Length>=20;}catch{}
    if(!present)failed.Add(new Challenge{Id="integrity:"+relative,Title="Supporting source or tests missing / emptied",Category="Project integrity",Path=relative,Hint=r.Level>=2?"Restore the supporting file and retain your incident repairs.":"Restore this supporting file from the trusted baseline while retaining your incident repairs."});
   }
   return failed;
  }
  public static bool Git(string dir,string args) {
   // MCP invokes tools on the host's worker pool. Dedicated pipe readers avoid
   // starving that pool while Git waits for stdout/stderr to be drained.
   // Generation never needs terminal input, user hooks, or a signing prompt.
   try {using(var p=new Process()) {
    p.StartInfo=new ProcessStartInfo("git","-c core.hooksPath=/dev/null -c commit.gpgsign=false "+args) {
     WorkingDirectory=dir,UseShellExecute=false,CreateNoWindow=true,
     RedirectStandardInput=true,RedirectStandardOutput=true,RedirectStandardError=true
    };
    p.StartInfo.Environment["GIT_TERMINAL_PROMPT"]="0";
    p.StartInfo.Environment["GCM_INTERACTIVE"]="Never";
    p.Start();p.StandardInput.Close();
    var stdout=new System.Threading.Thread(()=>{try{p.StandardOutput.ReadToEnd();}catch{}}){IsBackground=true};
    var stderr=new System.Threading.Thread(()=>{try{p.StandardError.ReadToEnd();}catch{}}){IsBackground=true};
    stdout.Start();stderr.Start();
    bool exited=p.WaitForExit(10000);
    if(!exited){p.Kill(true);p.WaitForExit();}
    stdout.Join(1000);stderr.Join(1000);
    return exited&&p.ExitCode==0;
   }} catch{return false;}
  }
 }
 public static class SelfTest {
  public static int Run() {
   string log=Path.Combine(AppDomain.CurrentDomain.BaseDirectory,"test-results.txt"),root=Path.Combine(Path.GetTempPath(),"SourceDefender-tests-"+Guid.NewGuid().ToString("N"));int checks=0;
   try {
    foreach(string language in Catalog.Languages) for(int level=0;level<4;level++) {
     foreach(int version in new[]{0,ProjectTemplates.Version})foreach(var c in ProjectTemplates.CatalogFor(language,level,version)) {if(!c.Check(c.Safe))throw new Exception(language+" clean control failed: "+c.Id);if(c.Check(c.Unsafe))throw new Exception(language+" attack passed: "+c.Id);if(c.Check(""))throw new Exception("Empty file passed");checks+=3;}
     var r=Engine.Generate(root,level,language,Scenario.All[0],"repeatable-seed");
     if(Engine.Scan(r).Count!=Difficulty.All[level].Bugs)throw new Exception("Attack count mismatch");
     if(r.RequiredFiles.Count<12)throw new Exception("Supporting project not generated: "+language);
     if(Directory.GetFiles(r.Folder,"*",SearchOption.AllDirectories).Where(Engine.IsSource).Sum(p=>File.ReadAllLines(p).Length)<750)throw new Exception("Generated codebase is too small: "+language);
     foreach(var c in ProjectTemplates.CatalogFor(language,level,r.TemplateVersion)) File.WriteAllText(Path.Combine(r.Folder,c.Path),c.Safe);
     if(Engine.Scan(r).Count!=0)throw new Exception("Remediation did not pass");
     string required=Path.Combine(r.Folder,r.RequiredFiles[0]);string original=File.ReadAllText(required);File.WriteAllText(required,"");if(Engine.Scan(r).Count!=1)throw new Exception("Emptied supporting file passed");File.WriteAllText(required,original);
     File.Delete(Path.Combine(r.Folder,Catalog.Build(language,level)[0].Path));if(Engine.Scan(r).Count!=1)throw new Exception("Deleted file passed");
     File.Delete(Path.Combine(Store.Root,r.Id+".json")); checks+=6;
    }
    File.WriteAllText(log,"PASS: "+checks+" checks across 4 languages, 4 difficulty levels, and legacy/current templates. Extensive generation, attack detection, layered encodings, repair, source size, and project-integrity checks passed.");return 0;
   } catch(Exception e){File.WriteAllText(log,"FAIL: "+e);return 1;}
   finally {if(Directory.Exists(root)) {foreach(string file in Directory.GetFiles(root,"*",SearchOption.AllDirectories))File.SetAttributes(file,FileAttributes.Normal);Directory.Delete(root,true);}}
  }
 }
}
