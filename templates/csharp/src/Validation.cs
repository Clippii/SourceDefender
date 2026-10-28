using System;
using System.Linq;
using System.Text.RegularExpressions;

namespace Service {
    public static class Validation {
        public static string Identifier(string value) {
            if (value == null || !Regex.IsMatch(value, "^[a-z][a-z0-9-]{1,63}$"))
                throw new ServiceError("Expected lowercase identifier", 400);
            return value;
        }

        public static string Title(string value) {
            if (value == null) throw new ServiceError("Title is required", 400);
            string clean = Regex.Replace(value.Trim(), @"\s+", " ");
            if (clean.Length < 3 || clean.Length > 160)
                throw new ServiceError("Title length out of range", 400);
            return clean;
        }

        public static int Money(int value) {
            if (value < 0 || value > 100000000) throw new ServiceError("Amount out of range", 400);
            return value;
        }

        public static Record Payload(Record input, Settings settings) {
            if (!settings.record_kinds.Contains(input.kind))
                throw new ServiceError("Unsupported record kind", 400);
            if (input.tags.Length > 10) throw new ServiceError("Too many tags", 400);
            return new Record { id=Identifier(input.id), kind=input.kind, owner=Identifier(input.owner),
                title=Title(input.title), amount=Money(input.amount),
                tags=input.tags.Select(Identifier).Distinct().OrderBy(x=>x).ToArray() };
        }
    }
}
