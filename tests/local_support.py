"""Run tests against an isolated instance of the actual local server."""
import http.client
import json
import os
import pathlib
import shutil
import signal
import subprocess
import tempfile
import time
import uuid

ROOT = pathlib.Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"
MODELS = ("ggml-base-q5_1.bin", "ggml-tiny-q5_1.bin", "ggml-silero-v6.2.0.bin")
NATIVE_SUFFIX = ".exe" if os.name == "nt" else ""


class TestServer:
    def __init__(self, executable, models, web=None, data=None, prefix=(), data_arg=None):
        self.executable = pathlib.Path(executable).resolve()
        self.web = pathlib.Path(web).resolve() if web else None
        self.prefix = tuple(prefix)
        self.keep = data is not None
        self.data = pathlib.Path(data or tempfile.mkdtemp(prefix="localscribe-test-")).resolve()
        self.data_arg = data_arg or str(self.data)
        self.data.mkdir(exist_ok=True)
        (self.data / "models").mkdir(exist_ok=True)
        for model in pathlib.Path(models).glob("*.bin"):
            if model.name in MODELS or model.name == "ggml-small-q5_1.bin":
                shutil.copyfile(model, self.data / "models" / model.name)
        for model in MODELS:
            if not (self.data / "models" / model).is_file():
                raise RuntimeError(f"Missing fixture model: {model}")
        self.process = None

    def start(self):
        command = [*self.prefix, str(self.executable), "--no-open", "--port", "0", "--data-dir", self.data_arg]
        if self.web:
            command += ["--web-root", str(self.web)]
        self.log = (self.data / "server.log").open("w+", encoding="utf-8")
        options = {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP} if os.name == "nt" else {}
        self.process = subprocess.Popen(command, stdout=self.log, stderr=subprocess.STDOUT, **options)
        cutoff = time.monotonic() + 45
        while self.process.poll() is None and time.monotonic() < cutoff:
            self.log.seek(0)
            for line in self.log:
                if line.startswith("LocalScribe listening at http://127.0.0.1:"):
                    self.url = line.split()[-1]
                    self.port = int(self.url.rsplit(":", 1)[1])
                    return self
            time.sleep(0.1)
        self.log.seek(0)
        raise RuntimeError("Server did not start. " + self.log.read()[-2000:])

    def stop(self):
        if self.process:
            if self.process.poll() is None:
                if os.name == "nt":
                    self.process.send_signal(signal.CTRL_BREAK_EVENT)
                else:
                    self.process.terminate()
                try:
                    self.process.wait(timeout=12)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait(timeout=8)
            self.process = None
            self.log.close()

    def __enter__(self):
        try:
            return self.start()
        except BaseException:
            self.__exit__()
            raise

    def __exit__(self, *_):
        self.stop()
        if not self.keep:
            shutil.rmtree(self.data, ignore_errors=True)

    def raw(self, path, method="GET", body=None, headers=None, timeout=20):
        sent_headers = {"X-LocalScribe": "1"} if headers is None else headers
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=timeout)
        try:
            connection.request(method, "/api" + path, body=body, headers=sent_headers)
            response = connection.getresponse()
            return response.status, dict(response.getheaders()), response.read()
        finally:
            connection.close()

    def api(self, path, method="GET", data=None, expected=200, timeout=20):
        body = json.dumps(data).encode() if data is not None else None
        status, _, content = self.raw(path, method, body, {"X-LocalScribe": "1", "Content-Type": "application/json"}, timeout)
        assert status == expected, (path, expected, status, content[:700])
        return json.loads(content) if content else None

    def import_file(self, path, options=None, name=None, expected=202):
        boundary = f"LocalScribeTest{uuid.uuid4().hex}"
        name = name or path.name
        fields = (
            f'--{boundary}\r\nContent-Disposition: form-data; name="options"\r\n\r\n{json.dumps(options or {})}\r\n'
            f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="{name}"\r\n'
            'Content-Type: application/octet-stream\r\n\r\n'
        )
        body = fields.encode() + path.read_bytes() + f"\r\n--{boundary}--\r\n".encode()
        status, _, content = self.raw("/jobs", "POST", body,
                                      {"X-LocalScribe": "1", "Content-Type": f"multipart/form-data; boundary={boundary}"})
        assert status == expected, (name, expected, status, content[:700])
        return json.loads(content)

    def wait(self, job, predicate=None, timeout=240):
        if predicate is None:
            predicate = lambda detail: detail["job"]["status"] not in ("running", "queued")
        cutoff = time.monotonic() + timeout
        while time.monotonic() < cutoff:
            detail = self.api("/jobs/" + job["id"])
            if predicate(detail):
                return detail
            if detail["job"]["status"] == "failed":
                raise AssertionError(detail["job"]["error"])
            time.sleep(0.18)
        raise TimeoutError(f"Timed out with job status: {detail['job']}")


def ffmpeg(*args):
    subprocess.run([os.environ.get("LOCALSCRIBE_FFMPEG", "ffmpeg"), "-nostdin", "-v", "error", "-y", *map(str, args)], check=True)


def arguments(parser):
    parser.add_argument("--binary", type=pathlib.Path, default=ROOT / "target" / "release" / f"localscribe{NATIVE_SUFFIX}")
    parser.add_argument("--models", type=pathlib.Path, required=True)
    parser.add_argument("--web", type=pathlib.Path)
    parser.add_argument("--data", type=pathlib.Path)
    return parser
