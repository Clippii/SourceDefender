"""A transport-neutral request router; no HTTP server is started."""
from .models import Response
from .repository import ConflictError


class Router:
    def __init__(self, application):
        self.application = application
        self.handlers = {
            'records.list': lambda session, body: application.list(
                session, body.get('offset', 0), body.get('limit', 20)),
            'records.create': lambda session, body: application.create(
                session, body['record'], body.get('request_key')),
            'records.transition': lambda session, body: application.transition(
                session, body['id'], body['target'], body['version']),
            'records.invoice': lambda session, body: application.invoice(session, body['id']),
            'records.export': lambda session, body: application.export(session),
        }

    def dispatch(self, operation, session, body=None):
        handler = self.handlers.get(operation)
        if handler is None:
            return Response(404, errors=['Unknown operation'])
        try:
            return handler(session, body or {})
        except PermissionError as error:
            return Response(403, errors=[str(error)])
        except ConflictError as error:
            return Response(409, errors=[str(error)])
        except KeyError as error:
            return Response(404, errors=[str(error)])
        except (ValueError, TypeError) as error:
            return Response(400, errors=[str(error)])
