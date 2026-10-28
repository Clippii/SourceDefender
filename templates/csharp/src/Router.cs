using System;
using System.Collections.Generic;

namespace Service {
    public sealed class Router {
        readonly Application application;
        public Router(Application application) { this.application=application; }

        public object Dispatch(string operation,Session session,Dictionary<string,object> body) {
            try {
                object result;
                switch(operation) {
                    case "records.list": result=application.List(session,0,20);break;
                    case "records.invoice": result=application.Invoice(session,(string)body["id"]);break;
                    case "records.export": result=application.Export(session);break;
                    case "records.transition":
                        result=application.Transition(session,(string)body["id"],(string)body["target"],Convert.ToInt32(body["version"]));break;
                    default:return new {status=404,errors=new[]{"Unknown operation"}};
                }
                return new {status=200,data=result};
            } catch(ServiceError error) {return new {status=error.Status,errors=new[]{error.Message}};}
            catch(KeyNotFoundException) {return new {status=400,errors=new[]{"Required field missing"}};}
        }
    }
}
