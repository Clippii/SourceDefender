using System;
using System.IO;
using System.Linq;
using System.Collections.Generic;

namespace SourceDefender {
    // One game service owns every rule, regardless of whether a human or MCP client calls it.
    public static class GameService {
        public const string Human="human", Agent="agent_practice";

        public static object PublicRun(Run r) {
            if(r==null)return null;
            return new {r.Id,r.Folder,r.Scenario,r.Language,r.Seed,r.StartedUtc,r.FinishedUtc,r.Status,
                r.Level,r.PenaltySeconds,r.Attempts,r.Hints,r.Events,r.Score,r.Seconds,
                AttackCount=r.Broken.Count,Mode=r.Mode??Human,r.ToolCalls,
                Findings=r.Findings??new List<PublicFinding>()};
        }

        public static object Snapshot() {
            using(Store.AcquireLock())return new {Runs=Store.Load().Select(PublicRun).ToArray(),
                Scenarios=Scenario.All,Difficulties=Difficulty.All,Languages=Catalog.Languages};
        }

        public static Run Start(int scenario,string language,int difficulty,string seed,string folder,string mode) {
            // Validate all arguments before creating folders, generating code, or changing records.
            if(scenario<0||scenario>=Scenario.All.Length)throw new ArgumentException("scenario must be an integer from 0 to 5.");
            if(difficulty<0||difficulty>=Difficulty.All.Length)throw new ArgumentException("difficulty must be an integer from 0 to 3.");
            if(language==null||!Catalog.Languages.Contains(language))throw new ArgumentException("language must be Python, JavaScript, Go, or C#.");
            if(seed!=null&&seed.Length>80)throw new ArgumentException("seed must contain at most 80 characters.");
            if(string.IsNullOrWhiteSpace(folder)||!Path.IsPathFullyQualified(folder))throw new ArgumentException("folder must be an absolute parent directory path.");
            string parent=Path.GetFullPath(folder);
            string records=Path.GetFullPath(Store.Root).TrimEnd(Path.DirectorySeparatorChar,Path.AltDirectorySeparatorChar);
            var comparison=OperatingSystem.IsWindows()?StringComparison.OrdinalIgnoreCase:StringComparison.Ordinal;
            if(parent.Equals(records,comparison)||parent.StartsWith(records+Path.DirectorySeparatorChar,comparison))
                throw new ArgumentException("Choose a repository parent folder outside the game records directory.");
            using(Store.AcquireLock()) {
                if(Store.Load().Any(r=>r.Status=="Active"&&(r.Mode??Human)==mode))
                    throw new InvalidOperationException("End the active "+(mode==Agent?"agent practice":"human")+" incident before starting another.");
                return Engine.Generate(parent,difficulty,language,Scenario.All[scenario],seed,mode);
            }
        }

        public static string Brief(Run run) {
            return run.Events.Last()+"\n\nRead README.md, docs/architecture.md, and HANDBOOK.md. Trace the application and policy services, investigate in your editor, save repairs, then validate.";
        }

        public static Run Status(string id,string mode,bool countToolCall) {
            using(Store.AcquireLock()) {
                var runs=Store.Load().Where(r=>(r.Mode??Human)==mode).ToList();
                var run=string.IsNullOrEmpty(id)?runs.FirstOrDefault(r=>r.Status=="Active")??runs.FirstOrDefault():runs.FirstOrDefault(r=>r.Id==id);
                if(run==null&&!string.IsNullOrEmpty(id))throw new ArgumentException("No matching "+mode+" incident exists.");
                if(run!=null&&countToolCall) {run.ToolCalls++;Store.Save(run);}
                return run;
            }
        }

        public static (Run Run,string Report) Act(string id,string mode,string command) {
            using(Store.AcquireLock()) {
                var r=Store.Load().FirstOrDefault(x=>x.Id==id&&(x.Mode??Human)==mode);
                if(r==null)throw new ArgumentException("No matching "+mode+" incident exists.");
                if(r.Status!="Active")throw new InvalidOperationException("This incident has already ended.");
                r.Findings=new List<PublicFinding>();
                string report;
                if(command=="end") {
                    r.Status="Abandoned";r.FinishedUtc=DateTime.UtcNow.ToString("o");
                    report="Run ended. Your repository remains on disk.";
                } else if(command=="validate") {
                    var failed=Engine.Scan(r);r.Attempts++;
                    if(failed.Count==0) {
                        r.Status="Contained";r.FinishedUtc=DateTime.UtcNow.ToString("o");
                        report="INCIDENT CONTAINED\n\nAll 16 behavior controls passed. Your repository stays on disk. Review its Git diff for a debrief.";
                    } else {
                        r.PenaltySeconds+=Difficulty.All[r.Level].Penalty;
                        report="VALIDATION FAILED / "+failed.Count+" controls unresolved\n+"+Difficulty.All[r.Level].Penalty+" seconds\n\n";
                        if(r.Level==0)foreach(var c in failed) {
                            report+=c.Title+"\n"+c.Path+"\n\n";
                            r.Findings.Add(new PublicFinding {Title=c.Title,Category=c.Category,Path=c.Path,Count=1});
                        }
                        else if(r.Level==1)foreach(var c in failed) {
                            report+="["+c.Category+"] "+c.Title+"\n";
                            r.Findings.Add(new PublicFinding {Title=c.Title,Category=c.Category,Path=null,Count=1});
                        }
                        else if(r.Level==2)foreach(var g in failed.GroupBy(c=>c.Category)) {
                            report+=g.Key+": "+g.Count()+"\n";
                            r.Findings.Add(new PublicFinding {Title=g.Key+" controls unresolved",Category=g.Key,Path=null,Count=g.Count()});
                        }
                        else {
                            report+="Detailed findings are disabled at Zero trust. Trace the changes.\n";
                            r.Findings.Add(new PublicFinding {Title=failed.Count+" controls unresolved",Category="Validation",Path=null,Count=failed.Count});
                        }
                        report+="\nMissing files and regressions also fail validation. Save changes before scanning again.";
                    }
                } else if(command=="hint") {
                    var failed=Engine.Scan(r);var c=failed.FirstOrDefault(x=>!r.Revealed.Contains(x.Id));
                    if(c==null)report=failed.Count==0?"All controls are clean. Validate to finish.":"Hints have already been provided for every remaining control.";
                    else {
                        r.Hints++;r.PenaltySeconds+=Difficulty.All[r.Level].HintCost;r.Revealed.Add(c.Id);
                        r.Findings.Add(new PublicFinding {Title=c.Title,Category=c.Category,Path=c.Path,Count=1});
                        report="HINT / +"+Difficulty.All[r.Level].HintCost+" seconds\n\n"+c.Title+"\n"+c.Path+"\n\n"+c.Hint;
                    }
                } else throw new ArgumentException("Unknown game action.");
                if(mode==Agent)r.ToolCalls++;
                r.Events.Add(report);Store.Save(r);return(r,report);
            }
        }
    }
}
