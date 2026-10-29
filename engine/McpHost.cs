using System;
using System.Linq;
using System.ComponentModel;
using System.ComponentModel.DataAnnotations;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using ModelContextProtocol.Server;
using ModelContextProtocol.Protocol;
using ModelContextProtocol;

namespace SourceDefender {
    public static class McpHost {
        public static int Run() {
            var builder=Host.CreateApplicationBuilder(Array.Empty<string>());
            builder.Logging.ClearProviders();
            builder.Logging.AddConsole(options=>options.LogToStandardErrorThreshold=LogLevel.Trace);
            builder.Logging.SetMinimumLevel(LogLevel.Warning);
            builder.Services.AddMcpServer(options=> {
                options.ServerInfo=new Implementation {Name="SourceDefender",Version="0.6.0"};
                options.ServerInstructions="SourceDefender agent practice. Use list_scenarios then start_run. Investigate and edit the returned local repository with your own file tools, save changes, and call validate_changes. Hints and unsuccessful validation add time. Runs are practice exercises, not a secure benchmark. This server never executes generated code. The desktop Agent practice view can observe progress. Human incidents are separate.";
            }).WithStdioServerTransport().WithTools<PracticeTools>(new JsonSerializerOptions(McpJsonUtilities.DefaultOptions) {
                NumberHandling=JsonNumberHandling.Strict
            });
            using(var host=builder.Build())host.Run();
            return 0;
        }
    }

    [McpServerToolType]
    public sealed class PracticeTools {
        static CallToolResult Result(Func<object> operation) {
            try {
                object data=operation();
                var structured=JsonSerializer.SerializeToElement(data);
                return new CallToolResult {
                    Content=new[]{new TextContentBlock {Text=structured.GetRawText()}},
                    StructuredContent=structured
                };
            } catch(Exception error) {
                return new CallToolResult {IsError=true,Content=new[]{new TextContentBlock {Text=error.Message}}};
            }
        }

        static object View(Run r) {
            if(r==null)return null;
            return new {id=r.Id,folder=r.Folder,scenario=r.Scenario,language=r.Language,
                difficulty=r.Level,difficulty_name=Difficulty.All[r.Level].Name,status=r.Status,seed=r.Seed,
                elapsed_seconds=r.Seconds,penalty_seconds=r.PenaltySeconds,validation_attempts=r.Attempts,
                hints=r.Hints,attack_count=r.Broken.Count,mode=GameService.Agent,tool_calls=r.ToolCalls,score=r.Score};
        }

        [McpServerTool(Name="list_scenarios",ReadOnly=true,Destructive=false,OpenWorld=false)]
        [Description("List available incident scenarios, languages, difficulty settings, and time penalties. No repository is created.")]
        public static CallToolResult ListScenarios() {return Result(()=>new {
            scenarios=Scenario.All.Select((s,i)=>new{id=i,name=s.Name,description=s.Description,mission=s.Mission}).ToArray(),
            languages=Catalog.Languages,
            difficulties=Difficulty.All.Select((d,i)=>new{id=i,name=d.Name,attack_count=d.Bugs,
                failed_validation_penalty_seconds=d.Penalty,hint_penalty_seconds=d.HintCost,encoding_layers=i}).ToArray(),
            mode=GameService.Agent
        });}

        [McpServerTool(Name="start_run",Destructive=false,OpenWorld=false)]
        [Description("Start one local agent-practice incident. Creates a unique repository subfolder and starts the timer immediately. At most one agent incident is active in this data directory; human runs are separate. Use your own file tools on the returned folder.")]
        public static CallToolResult StartRun(
            [Description("Scenario ID from list_scenarios, 0 to 5."),Range(0,5)] int scenario,
            [Description("Python, JavaScript, Go, or C#.")] string language,
            [Description("Difficulty ID: 0 Intern, 1 Maintainer, 2 Incident commander, 3 Zero trust."),Range(0,3)] int difficulty,
            [Description("Absolute parent folder for a newly generated incident subfolder.")] string folder,
            [Description("Optional reproducible seed, at most 80 characters.")] string seed=null) {
            return Result(()=> {
                var run=GameService.Start(scenario,language,difficulty,seed,folder,GameService.Agent);
                return new {run=View(run),report=GameService.Brief(run)};
            });
        }

        [McpServerTool(Name="get_run_status",Destructive=false,OpenWorld=false)]
        [Description("Get an agent-practice run's status, elapsed time, penalties and latest feedback. Without run_id, returns the active or most recent agent run. Counts this MCP tool call; never returns attack IDs or private game metadata.")]
        public static CallToolResult GetRunStatus([Description("Agent run ID; omit for the active or latest agent run.")] string run_id=null) {
            return Result(()=> {
                var r=GameService.Status(run_id,GameService.Agent,true);
                return new {run=View(r),report=r==null?"No agent-practice incidents yet.":r.Events.LastOrDefault()??""};
            });
        }

        static CallToolResult Act(string id,string command) {return Result(()=> {
            var result=GameService.Act(id,GameService.Agent,command);
            return new {run=View(result.Run),report=result.Report};
        });}

        [McpServerTool(Name="validate_changes",Destructive=false,OpenWorld=false)]
        [Description("Validate saved files in an active agent run. A failed validation adds the difficulty's time penalty. Passing validation ends the run. Feedback follows the same difficulty rules as human play. Does not execute project code.")]
        public static CallToolResult ValidateChanges([Description("Active agent run ID.")] string run_id) {return Act(run_id,"validate");}

        [McpServerTool(Name="request_hint",Destructive=false,OpenWorld=false)]
        [Description("Reveal one hint for an unresolved control, adding the difficulty's hint time penalty. Repeated hints for already revealed controls are not charged.")]
        public static CallToolResult RequestHint([Description("Active agent run ID.")] string run_id) {return Act(run_id,"hint");}

        [McpServerTool(Name="end_run",Destructive=true,OpenWorld=false)]
        [Description("End an active agent incident as abandoned. Files stay on disk; this does not delete the generated repository or affect human runs.")]
        public static CallToolResult EndRun([Description("Active agent run ID to abandon.")] string run_id) {return Act(run_id,"end");}
    }
}
