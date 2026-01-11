"""Tests for file uploads and multipart form data (Phase 4)"""
import unittest
import sys
import os
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))
import requestx
from testcontainers.generic import ServerContainer


class HttpbinTestCase(unittest.TestCase):
    """Base test case with httpbin container."""

    @classmethod
    def setUpClass(cls):
        cls.container = ServerContainer(port=80, image="kennethreitz/httpbin")
        cls.container.start()
        cls.httpbin_port = cls.container.get_exposed_port(80)
        global HTTPBIN_HOST
        HTTPBIN_HOST = f"http://localhost:{cls.httpbin_port}"

    @classmethod
    def tearDownClass(cls):
        cls.container.stop()


class TestFileUpload(HttpbinTestCase):
    """Tests for file upload functionality."""

    def test_file_upload_single(self):
        """Test single file upload with file object."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as f:
            f.write("Hello, World!")
            f.flush()
            fname = f.name
            
        try:
            with open(fname, 'rb') as f:
                files = {'file': f}
                r = requestx.post(f"{HTTPBIN_HOST}/post", files=files)
            self.assertEqual(r.status_code, 200)
            data = r.json()
            self.assertIn("files", data)
        finally:
            os.unlink(fname)

    def test_file_upload_with_filename_tuple(self):
        """Test file upload with explicit filename tuple."""
        data = b"Test file content"
        files = {'upload': ('test.txt', data)}
        r = requestx.post(f"{HTTPBIN_HOST}/post", files=files)
        self.assertEqual(r.status_code, 200)
        data = r.json()
        self.assertIn("files", data)

    def test_file_upload_with_content_type(self):
        """Test file upload with explicit content type."""
        data = b'{"key": "value"}'
        files = {'file': ('data.json', data, 'application/json')}
        r = requestx.post(f"{HTTPBIN_HOST}/post", files=files)
        self.assertEqual(r.status_code, 200)

    def test_file_upload_multiple_files(self):
        """Test multiple file upload."""
        files = [
            ('file1', ('test1.txt', b'Content 1')),
            ('file2', ('test2.txt', b'Content 2')),
        ]
        r = requestx.post(f"{HTTPBIN_HOST}/post", files=files)
        self.assertEqual(r.status_code, 200)
        data = r.json()
        self.assertIn("files", data)


class TestMultipartFormData(HttpbinTestCase):
    """Tests for multipart form data."""

    def test_multipart_with_data_and_files(self):
        """Test multipart form with both data fields and files."""
        files = {'file': ('test.txt', b'Test content')}
        data = {'description': 'Test description'}
        r = requestx.post(f"{HTTPBIN_HOST}/post", files=files, data=data)
        self.assertEqual(r.status_code, 200)

    def test_multipart_form_data_only(self):
        """Test multipart form with only data (no files)."""
        data = {'name': 'Test User', 'email': 'test@example.com'}
        r = requestx.post(f"{HTTPBIN_HOST}/post", data=data)
        # Without files, this should be regular form data
        self.assertEqual(r.status_code, 200)


class TestFileUploadEdgeCases(HttpbinTestCase):
    """Edge case tests for file uploads."""

    def test_file_upload_empty_data(self):
        """Test file upload with empty file content."""
        files = {'file': ('empty.txt', b'')}
        r = requestx.post(f"{HTTPBIN_HOST}/post", files=files)
        self.assertEqual(r.status_code, 200)

    def test_file_upload_binary_content(self):
        """Test file upload with binary content."""
        binary_data = bytes(range(256))
        files = {'file': ('binary.bin', binary_data)}
        r = requestx.post(f"{HTTPBIN_HOST}/post", files=files)
        self.assertEqual(r.status_code, 200)
        data = r.json()
        self.assertIn("files", data)

    def test_file_upload_image(self):
        """Test file upload with image content type."""
        # Create a minimal PNG header (1x1 transparent pixel)
        png_data = b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\xcf\xc0\x00\x00\x00\x03\x00\x01\x00\x05\xfe\xd4\x00\x00\x00\x00IEND\xaeB`\x82'
        files = {'image': ('pixel.png', png_data, 'image/png')}
        r = requestx.post(f"{HTTPBIN_HOST}/post", files=files)
        self.assertEqual(r.status_code, 200)

    def test_file_upload_with_form(self):
        """Test file upload with additional form fields."""
        files = {'document': ('doc.txt', b'Document content')}
        data = {'title': 'My Document', 'author': 'Test Author'}
        r = requestx.post(f"{HTTPBIN_HOST}/post", files=files, data=data)
        self.assertEqual(r.status_code, 200)

    def test_file_upload_get_request(self):
        """Test that files parameter also works with GET requests."""
        # Note: GET with files is unusual but should be supported
        files = {'file': ('test.txt', b'Content')}
        # GET requests with files typically send them as query params
        # httpbin's /get endpoint should handle this
        r = requestx.get(f"{HTTPBIN_HOST}/get", files=files)
        self.assertEqual(r.status_code, 200)


class TestFileUploadWithSession(HttpbinTestCase):
    """Tests for file uploads with Session."""

    def test_session_file_upload(self):
        """Test file upload with Session."""
        session = requestx.Session()
        files = {'file': ('test.txt', b'Session file upload')}
        r = session.post(f"{HTTPBIN_HOST}/post", files=files)
        self.assertEqual(r.status_code, 200)

    def test_session_multipart_with_data(self):
        """Test multipart form with Session."""
        session = requestx.Session()
        files = {'file': ('data.csv', b'col1,col2\n1,2\n3,4')}
        data = {'user': 'testuser'}
        r = session.post(f"{HTTPBIN_HOST}/post", files=files, data=data)
        self.assertEqual(r.status_code, 200)


if __name__ == "__main__":
    unittest.main()
