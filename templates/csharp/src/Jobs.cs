using System;
using System.Collections.Generic;

namespace Service {
    public sealed class Job {
        public string Id,Kind,Error,State="pending";
        public int Attempts;
        public DateTime RetryAt=DateTime.MinValue;
        public Dictionary<string,object> Payload=new Dictionary<string,object>();
    }

    public sealed class JobQueue {
        readonly Queue<Job> pending=new Queue<Job>();
        readonly HashSet<string> known=new HashSet<string>();
        public readonly List<Job> Completed=new List<Job>();
        public readonly List<Job> DeadLetters=new List<Job>();
        readonly int maxAttempts;

        public JobQueue(int maxAttempts) {this.maxAttempts=maxAttempts;}
        public bool Enqueue(Job job) {
            if(!known.Add(job.Id))return false;
            pending.Enqueue(job);return true;
        }

        public Job RunOnce(Dictionary<string,Action<Dictionary<string,object>>> handlers,DateTime now) {
            if(pending.Count==0)return null;
            var job=pending.Dequeue();
            if(job.RetryAt>now) {pending.Enqueue(job);return null;}
            job.Attempts++;
            try {
                handlers[job.Kind](job.Payload);job.State="completed";Completed.Add(job);
            } catch(Exception error) {
                job.Error=error.Message;
                if(job.Attempts>=maxAttempts) {job.State="failed";DeadLetters.Add(job);}
                else {job.State="retry";job.RetryAt=now.AddSeconds(Math.Pow(2,job.Attempts));pending.Enqueue(job);}
            }
            return job;
        }

        public object Statistics() {return new {pending=pending.Count,completed=Completed.Count,deadLetters=DeadLetters.Count};}
    }
}
