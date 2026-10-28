using System;
using System.IO;
using System.Linq;
using System.Text;
using System.Collections.Generic;
using System.Reflection;
namespace SourceDefender {
 public static class ProjectTemplates {
  public const int Version=2;
  static string Key(string language) {return new[]{"python","javascript","go","csharp"}[Array.IndexOf(Catalog.Languages,language)];}
  static string ResourceName(string name) {return Assembly.GetExecutingAssembly().GetManifestResourceNames().FirstOrDefault(n=>n.Replace('\\','/')==name)??name;}
  static string Read(string name) {using(var stream=Assembly.GetExecutingAssembly().GetManifestResourceStream(ResourceName(name))) {if(stream==null)throw new Exception("Missing embedded project template: "+name);using(var reader=new StreamReader(stream))return reader.ReadToEnd();}}
  public static List<Challenge> CatalogFor(string language,int level,int version) {
   var catalog=Catalog.Build(language,level);
   if(version<2)return catalog;
   foreach(var c in catalog.Take(8)) {
    string body=Read("templates/"+Key(language)+"/modules/"+c.Id+".part");
    c.Safe+="\n"+body;c.Unsafe+="\n"+body;
    string[] ids={"auth","telemetry","billing","audit","roles","updates","exports","plugins"};
    int index=Array.IndexOf(ids,c.Id);
    string[] python={"authenticate","collect_metrics","build_invoice","append_event","effective_principal","plan_update","plan_export","plugin_plan"};
    string[] js={"authenticate","collectMetrics","buildInvoice","appendEvent","effectivePrincipal","planUpdate","planExport","pluginPlan"};
    string[] go={"Authenticate","CollectMetrics","BuildInvoice","AppendEvent","EffectivePrincipal","PlanUpdate","PlanExport","PluginPlan"};
    string[] cs={"Authenticate","Collect","BuildInvoice","Append","EffectivePrincipal","Plan","Plan","Plan"};
    string api=(language=="Python"?"def "+python[index]:language=="JavaScript"?"function "+js[index]:language=="Go"?"func "+go[index]:cs[index])+"(";
    var prior=c.Check;c.Check=t=>prior(t)&&t.Contains(api);
   }
   return catalog;
  }
  public static Dictionary<string,string> Files(string language,int level,Scenario scenario) {
   var files=new Dictionary<string,string>();string prefix="templates/"+Key(language)+"/";
   foreach(string rawName in Assembly.GetExecutingAssembly().GetManifestResourceNames()) {
    string name=rawName.Replace('\\','/');
    string path=null;
    if(name.StartsWith(prefix))path=name.Substring(prefix.Length);
    else if(name.StartsWith("templates/common/"))path=name.Substring("templates/common/".Length);
    if(path==null||path.StartsWith("modules/"))continue;
    string body=Read(name).Replace("@@SCENARIO@@",scenario.Name).Replace("@@SLUG@@",scenario.Slug);
    var challenges=Catalog.Build(language,level);
    foreach(var c in challenges.Take(8))body=body.Replace("@@"+c.Id.ToUpperInvariant()+"@@",Path.GetFileNameWithoutExtension(c.Path));
    files[path]=body;
   }
   string[] domains;
   switch(scenario.Slug) {
    case "exchange":domains=new[]{"trade","settlement","portfolio","risk_review"};break;
    case "gameserver":domains=new[]{"inventory","match","season_reward","moderation"};break;
    case "careline":domains=new[]{"patient_record","appointment","referral","consent"};break;
    case "uplink":domains=new[]{"telemetry","command","pass_window","instrument"};break;
    case "package":domains=new[]{"package","version","dependency","publication"};break;
    default:domains=new[]{"release","deployment","approval","artifact"};break;
   }
   var json=new JavaScriptSerializer();var rows=new List<object>();
   for(int i=0;i<48;i++)rows.Add(new {id="rec-"+(i+1).ToString("000"),kind=domains[i%4],owner="user-"+(i%6+1),title=domains[i%4].Replace('_',' ')+" #"+(i+1),status=i%3==0?"pending":"active",version=1,amount=1250+i*75,tags=new[]{domains[i%4],i%2==0?"priority":"standard"}});
   files["fixtures/records.json"]=json.Serialize(rows)+"\n";
   files["config/scenario.json"]=json.Serialize(new {name=scenario.Name,slug=scenario.Slug,record_kinds=domains,workflows=new[]{"intake","review","approve","publish","reconcile"},page_size=20,max_export_rows=1000,retention_days=30})+"\n";
   files["fixtures/users.json"]=json.Serialize(Enumerable.Range(1,6).Select(i=>new {id="user-"+i,name="Analyst "+i,roles=i==1?new[]{"operator","reviewer"}:new[]{"viewer"},verified=true}).ToArray())+"\n";
   return files;
  }
 }
}
