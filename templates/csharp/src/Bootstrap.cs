using System;
using System.IO;
using System.Text.Json;

namespace Service {
    public static class Bootstrap {
        public static Application Build(string root) {
            var options=new JsonSerializerOptions {IncludeFields=true};
            var records=JsonSerializer.Deserialize<Record[]>(File.ReadAllText(Path.Combine(root,"fixtures/records.json")),options);
            var settings=JsonSerializer.Deserialize<Settings>(File.ReadAllText(Path.Combine(root,"config/scenario.json")),options);
            return new Application(new Repository(records),settings);
        }

        public static Session ExampleSession() {
            return new Session {Id="user-1",Verified=true,Token="fixture-session",
                Roles=new System.Collections.Generic.List<string>{"operator","reviewer"}};
        }
    }
}
