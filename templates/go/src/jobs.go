package service

import (
    "fmt"
    "time"
)

type Job struct {
    ID, Kind, State, Error string
    Payload map[string]interface{}
    Attempts int
    RetryAt time.Time
}

type JobQueue struct {
    Pending []*Job
    Completed []*Job
    DeadLetters []*Job
    known map[string]bool
    MaxAttempts int
}

func NewJobQueue(maxAttempts int) *JobQueue { return &JobQueue{known: map[string]bool{}, MaxAttempts: maxAttempts} }

func (q *JobQueue) Enqueue(job *Job) bool {
    if q.known[job.ID] { return false }; q.known[job.ID] = true
    job.State = "pending"; q.Pending = append(q.Pending, job); return true
}

func (q *JobQueue) RunOnce(handlers map[string]func(map[string]interface{}) error, now time.Time) *Job {
    if len(q.Pending) == 0 { return nil }
    job := q.Pending[0]; q.Pending = q.Pending[1:]
    if now.Before(job.RetryAt) { q.Pending = append(q.Pending, job); return nil }
    job.Attempts++
    handler, exists := handlers[job.Kind]
    var err error
    if !exists { err = fmt.Errorf("unknown job kind: %s", job.Kind) } else { err = handler(job.Payload) }
    if err == nil { job.State = "completed"; q.Completed = append(q.Completed, job); return job }
    job.Error = err.Error()
    if job.Attempts >= q.MaxAttempts { job.State = "failed"; q.DeadLetters = append(q.DeadLetters, job)
    } else { job.State = "retry"; job.RetryAt = now.Add(time.Duration(1<<job.Attempts)*time.Second); q.Pending = append(q.Pending, job) }
    return job
}
