package service

type Response struct { Status int; Data interface{}; Errors []string }
type Request struct { Operation string; Session Session; ID, Target string; Version, Offset, Limit int }

type Router struct { Application *Application }

func (r Router) Dispatch(request Request) Response {
    var data interface{}; var err error
    switch request.Operation {
    case "records.list": data, err = r.Application.List(request.Session, request.Offset, request.Limit)
    case "records.invoice": data, err = r.Application.Invoice(request.Session, request.ID)
    case "records.export": data, err = r.Application.Export(request.Session)
    case "records.transition": data, err = r.Application.Transition(request.Session, request.ID, request.Target, request.Version)
    default: return Response{Status: 404, Errors: []string{"unknown operation"}}
    }
    if err != nil { return Response{Status: 400, Errors: []string{err.Error()}} }
    return Response{Status: 200, Data: data, Errors: []string{}}
}
