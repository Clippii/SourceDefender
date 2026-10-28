using System;
using System.Collections.Generic;
using System.Text.Json;

namespace SourceDefender {
    // Preserve the existing local protocol and saved-run field names on modern .NET.
    public sealed class JavaScriptSerializer {
        static readonly JsonSerializerOptions Options = new JsonSerializerOptions {
            IncludeFields = true, PropertyNameCaseInsensitive = true
        };
        public string Serialize(object value) { return JsonSerializer.Serialize(value, Options); }
        public T Deserialize<T>(string value) {
            if (typeof(T) == typeof(Dictionary<string, object>)) {
                using (var document = JsonDocument.Parse(value))
                    return (T)ConvertValue(document.RootElement);
            }
            return JsonSerializer.Deserialize<T>(value, Options);
        }
        static object ConvertValue(JsonElement element) {
            switch (element.ValueKind) {
                case JsonValueKind.Object:
                    var result = new Dictionary<string, object>();
                    foreach (var property in element.EnumerateObject())
                        result[property.Name] = ConvertValue(property.Value);
                    return result;
                case JsonValueKind.Array:
                    var items = new List<object>();
                    foreach (var item in element.EnumerateArray()) items.Add(ConvertValue(item));
                    return items.ToArray();
                case JsonValueKind.String: return element.GetString();
                case JsonValueKind.Number: return element.GetInt32();
                case JsonValueKind.True: return true;
                case JsonValueKind.False: return false;
                default: return null;
            }
        }
    }
}
