using System;
using System.Collections.Generic;
using System.IO;

namespace SourceDefender {
    public class Difficulty {
        public string Name; public int Bugs; public int Penalty; public int HintCost; public string Description;
        public override string ToString() { return Name; }
        public static Difficulty[] All = {
            new Difficulty { Name="Intern", Bugs=3, Penalty=15, HintCost=20, Description="3 findings · +15s per failed scan\nFull findings with file locations. A gentle first incident." },
            new Difficulty { Name="Maintainer", Bugs=6, Penalty=45, HintCost=45, Description="6 findings · +45s per failed scan\nFinding titles and categories. Follow the evidence." },
            new Difficulty { Name="Incident commander", Bugs=10, Penalty=90, HintCost=75, Description="10 attacks · 2 encoding layers · +90s per failed scan\nCategory summaries. Misleading maintenance commits." },
            new Difficulty { Name="Zero trust", Bugs=16, Penalty=180, HintCost=120, Description="16 attacks · 3 encoding layers · +180s per failed scan\nOnly the remaining count. Every control needs your attention." }
        };
    }
    public class Scenario {
        public string Name, Slug, Description, Mission;
        public override string ToString() { return Name; }
        public static Scenario[] All = {
            new Scenario { Name="Midnight release", Slug="release", Description="A release train. An overpowered bot. One last review.", Mission="Protect a deployment service before the midnight release. Audit application trust boundaries and the repository's release controls." },
            new Scenario { Name="Neon exchange", Slug="exchange", Description="Trading infrastructure with expensive trust assumptions.", Mission="Take ownership of a trading gateway after a rushed acquisition. Secure account data, outbound connections, and the supply chain." },
            new Scenario { Name="Last-life game server", Slug="gameserver", Description="Player inventories, admin commands, and a rogue webhook.", Mission="Harden a multiplayer backend before the next season. Protect player records and stop untrusted input crossing privileged boundaries." },
            new Scenario { Name="Careline records", Slug="careline", Description="Sensitive records meet an inherited repository.", Mission="Prepare a records service for handover. Remove embedded credentials, defend data access, and require accountable changes." },
            new Scenario { Name="Orbital uplink", Slug="uplink", Description="Telemetry from orbit. A very terrestrial supply chain.", Mission="Audit a satellite telemetry relay before launch. The service must reject untrusted messages and protect its update pipeline." },
            new Scenario { Name="The abandoned package", Slug="package", Description="You inherited a popular package. So did its problems.", Mission="Rescue an abandoned automation package. Audit its helper modules, dependency source, and repository permissions before publishing." }
        };
    }
    public class PublicFinding {
        public string Title, Category, Path;
        public int Count;
    }
    public class Run {
        public string Id, Folder, Scenario, Language, Seed, StartedUtc, FinishedUtc, Status;
        public string Mode = "human";
        public int ToolCalls;
        public int Level, PenaltySeconds, Attempts, Hints;
        public int TemplateVersion;
        public List<string> RequiredFiles = new List<string>();
        public List<string> Broken = new List<string>();
        public List<string> Revealed = new List<string>();
        public List<string> Events = new List<string>();
        public List<PublicFinding> Findings = new List<PublicFinding>();
        public double Seconds {
            get { DateTime end = string.IsNullOrEmpty(FinishedUtc) ? DateTime.UtcNow : DateTime.Parse(FinishedUtc, null, System.Globalization.DateTimeStyles.RoundtripKind); return Math.Max(0, (end-DateTime.Parse(StartedUtc, null, System.Globalization.DateTimeStyles.RoundtripKind)).TotalSeconds)+PenaltySeconds; }
        }
        public int Score { get { return Math.Max(100, (Level+1)*5000-(int)Seconds-Attempts*50-Hints*100); } }
    }
    public static class Store {
        static Store() {}
        public static readonly string Root = DataRoot();
        static string DataRoot() {
            string configured = Environment.GetEnvironmentVariable("SOURCEDEFENDER_DATA_DIR");
            if (!string.IsNullOrWhiteSpace(configured)) return configured;
            if (OperatingSystem.IsLinux()) {
                string xdg = Environment.GetEnvironmentVariable("XDG_DATA_HOME");
                if (string.IsNullOrWhiteSpace(xdg)) xdg = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".local", "share");
                return Path.Combine(xdg, "SourceDefender");
            }
            return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "SourceDefender");
        }
        static JavaScriptSerializer json = new JavaScriptSerializer();
        public static IDisposable AcquireLock() {
            Directory.CreateDirectory(Root);
            var timer=System.Diagnostics.Stopwatch.StartNew();
            while(true) {
                try {return new FileStream(Path.Combine(Root,".store.lock"),FileMode.OpenOrCreate,FileAccess.ReadWrite,FileShare.None);}
                catch(IOException) {if(timer.Elapsed.TotalSeconds>=20)throw new InvalidOperationException("The local game store is busy. Try again shortly.");System.Threading.Thread.Sleep(40);}
            }
        }
        public static List<Run> Load() {
            Directory.CreateDirectory(Root);
            var result = new List<Run>();
            foreach (string p in Directory.GetFiles(Root, "*.json")) { try { var r=json.Deserialize<Run>(File.ReadAllText(p)); if(r!=null && r.Id!=null) result.Add(r); } catch { } }
            result.Sort((a,b)=>string.Compare(b.StartedUtc,a.StartedUtc,StringComparison.Ordinal)); return result;
        }
        public static void Save(Run r) {
            Directory.CreateDirectory(Root);
            string path=Path.Combine(Root,r.Id+".json"), temp=path+"."+Guid.NewGuid().ToString("N")+".tmp";
            File.WriteAllText(temp,json.Serialize(r));
            try {File.Move(temp,path,true);}finally {if(File.Exists(temp))File.Delete(temp);}
        }
    }
}
