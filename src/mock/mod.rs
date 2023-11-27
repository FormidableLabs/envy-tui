pub const TEST_JSON_1: &str = r#"{ 


    "type": "trace",
"data": {
"id":"1","type":"HttpRequest","timestamp":1694891653602,"http": { "timings": {
      "blocked": 1.701791,
      "dns": 37.977375,
      "connect": 38.259209,
      "state": "received",
      "send": 0.03825,
      "wait": 50.718333,
      "receive": 1.474667,
      "ssl": 21.786959
}, "duration":200,

      "state": "received",
"httpVersion":"1.1","method":"GET","host":"auth.restserver.com","port":443,"path":"/auth?client=mock_client","url":"http://auth.restserver.com/auth?client=mock_client","requestHeaders":{"Authorization":["Basic dXNlcm5hbWU6cGFzc3dvcmQ="],"Content-Type":["application/x-www-form-urlencoded"],"Accept":["*/*"],"Content-Length":["0"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"],"Connection":["close"]},"statusCode":200,"statusMessage":"OK","responseHeaders":{"content-type":"application/json","transfer-encoding":"chunked","connection":"close","cache-control":"no-store","content-encoding":"gzip","strict-transport-security":"max-age=31536000; includeSubDomains","vary":"Accept-Encoding, User-Agent","pragma":"no-cache"},"responseBody":"{\"access_token\":\"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.vqb33-7FqzFWPNlr0ElW1v2RjJRZBel3CdDHBWD7y_o\"}"
}
} }"#;

pub const TEST_JSON_2: &str = r#"{  

    "type": "trace",
"data": {
"id":"2","type":"HttpRequest","timestamp":1694948911169,
"http": {
      "state": "received",
"timings": {
      "blocked": 1.701791,
      "dns": 37.977375,
      "connect": 38.259209,
      "send": 0.03825,
      "wait": 50.718333,
      "receive": 1.474667,
      "ssl": 21.786959
},
"httpVersion":"1.1","method":"POST","host":"localhost","port":3000,"path":"/api/graphql","url":"http://localhost:3000/api/graphql","requestHeaders":{"authorization":"Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.vqb33-7FqzFWPNlr0ElW1v2RjJRZBel3CdDHBWD7y_o","content-type":["application/json"],"Accept":["*/*"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"]},"requestBody":"{\"query\":\"query People {\\n  people {\\n    id\\nfirstName\\nlastName\\n  }\\n}\\n\",\"operationName\":\"People\",\"variables\":{}}","statusCode":200,"statusMessage":"OK","responseHeaders":{"x-powered-by":"Express","cache-control":"private, no-store","surrogate-key":"all","access-control-allow-origin":"*","access-control-allow-credentials":"true","content-type":"application/json","content-length":"28","vary":"Accept-Encoding","date":"Thu, 17 Mar 2022 19:51:01 GMT","connection":"keep-alive","keep-alive":"timeout=5"},"responseBody":"{\"data\":{\"people\":[{\"id\":\"1\",\"firstName\":\"Peter\",\"lastName\":\"Piper\"},{\"id\":\"2\",\"firstName\":\"Tom\",\"lastName\":\"Thumb\"},{\"id\":\"3\",\"firstName\":\"Mary\",\"lastName\":\"Hadalittlelamb\"}]}}","duration":500

} }

    }"#;

pub const TEST_JSON_3: &str = r#"{
    "type": "trace",
    "data": {
"id":"3","type":"HttpRequest","timestamp":1694948912369,"http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"data.restserver.com","port":443,"path":"/features","url":"http://data.restserver.com/features","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"},"statusCode":200,"statusMessage":"OK","responseHeaders":{"content-type":"application/json; charset=utf-8","content-length":"351","date":"Thu, 17 Mar 2022 19:51:00 GMT","vary":"Origin","connection":"close"},"responseBody":"{\"awesomeFeature\":true,\"crappyFeature\":false}","duration":15

}    }
}"#;

pub const TEST_JSON_4: &str = r#"{

    "type": "trace",
"data": { "id":"4","type":"HttpRequest","timestamp":1694948915469, "http": { 


      "state": "received",
"httpVersion":"1.1","method":"GET","host":"data.restserver.com","port":443,"path":"/countries?start=0&count=20","url":"http://data.restserver.com/countries?start=0&count=20","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"},"statusCode":404,"statusMessage":"Not found","responseHeaders":{"content-type":"application/json; charset=utf-8","content-length":"2","date":"Thu, 17 Mar 2022 19:51:01 GMT","vary":"Origin","connection":"close"},"duration":10 } }
}"#;

pub const TEST_JSON_5: &str = r#"{
    "type": "trace",

    "data": {
"id":"5","type":"HttpRequest","timestamp":1694948931769, "http"  : {
      "state": "received",
"httpVersion":"1.1","method":"POST","host":"data.restserver.com","port":443,"path":"/people","url":"http://data.restserver.com/people","requestHeaders":{"authorization":"Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.vqb33-7FqzFWPNlr0ElW1v2RjJRZBel3CdDHBWD7y_o","content-type":["application/json"],"Accept":["*/*"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"]},"requestBody":"{\"firstName\":\"Paddington\",\"lastName\":\"Bear\"}","statusCode":200,"statusMessage":"OK","responseHeaders":{"cache-control":"private, no-store","surrogate-key":"all","access-control-allow-origin":"*","access-control-allow-credentials":"true","content-type":"application/json","content-length":"11","vary":"Accept-Encoding","date":"Thu, 17 Mar 2022 19:51:02 GMT","connection":"keep-alive","keep-alive":"timeout=5"},"responseBody":"{\"id\":\"4\"}","duration":1300

}   }


}"#;

pub const TEST_JSON_6: &str = r#"{
    "type": "trace",

    "data": {
"id":"6","type":"HttpRequest","timestamp":1694948931869, "http": {
      "state": "received",
"httpVersion":"1.1","method":"POST","host":"localhost","port":3000,"path":"/api/graphql","url":"http://localhost:3000/api/graphql","requestHeaders":{"authorization":"Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.vqb33-7FqzFWPNlr0ElW1v2RjJRZBel3CdDHBWD7y_o","content-type":["application/json"],"Accept":["*/*"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"]},"requestBody":"{\"query\":\"mutation RegisterPerson($id: String) {\\n  registerPerson(id: $id) {\\n    success\\n}\\n}\",\"type\":\"HttpRequest\",\"operationName\":\"RegisterPerson\",\"variables\":{\"id\":\"4\",\"type\":\"HttpRequest\"}}","statusCode":200,"statusMessage":"OK","responseHeaders":{"x-powered-by":"Express","cache-control":"private, no-store","surrogate-key":"all","access-control-allow-origin":"*","access-control-allow-credentials":"true","content-type":"application/json","content-length":"28","vary":"Accept-Encoding","date":"Thu, 17 Mar 2022 19:51:01 GMT","connection":"keep-alive","keep-alive":"timeout=5"},"responseBody":"{\"data\":{\"success\":true}}","duration":629

}    }

}"#;

pub const TEST_JSON_7: &str = r#"{

    "type": "trace",

    "data": {
"id":"7","type":"HttpRequest","timestamp":1694948935009, "http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"data.restserver.com","port":433,"path":"/movies?start=0&count=20","url":"https://data.restserver.com:433/movies?start=0&count=20","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"},"statusCode":500,"statusMessage":"Internal Server Error","responseHeaders":{"content-type":"application/json; charset=utf-8","content-length":"0","date":"Thu, 17 Mar 2022 19:51:01 GMT","vary":"Origin","connection":"close"},"duration":5000

}    }


}"#;

pub const TEST_JSON_8: &str = r#"{

    "type": "trace",

    "data":
     {
"id":"8","type":"HttpRequest","timestamp":1694948938149, "http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"hits.webstats.com","port":433,"path":"/?apikey=c82e66bd-4d5b-4bb7-b439-896936c94eb2","url":"https://hits.webstats.com:433/?apikey=c82e66bd-4d5b-4bb7-b439-896936c94eb2","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"},"statusCode":200,"statusMessage":"OK","responseHeaders":{"content-type":"application/xml; charset=utf-8","content-length":"55","date":"Thu, 17 Mar 2022 19:51:01 GMT","vary":"Origin","connection":"close"},"responseBody":"<hits><today>10</today><yesterday>15</yesterday></hits>","duration":5000
}
     }
}"#;

pub const TEST_JSON_9: &str = r#"{
    "type": "trace",

    "data": {
"id":"9","type":"HttpRequest","timestamp":1694948938549,"http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"data.restserver.com","port":433,"path":"/features","url":"https://data.restserver.com:433/features","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"
}}
    }
}"#;

pub const TEST_JSON_10: &str = r#"{
    "type": "trace",
    "data": {

"id":"10","type":"HttpRequest","timestamp":1694891653603,"http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"auth.restserver.com","port":443,"path":"/auth?client=mock_client","url":"http://auth.restserver.com/auth?client=mock_client","requestHeaders":{"Authorization":["Basic dXNlcm5hbWU6cGFzc3dvcmQ="],"Content-Type":["application/x-www-form-urlencoded"],"Accept":["*/*"],"Content-Length":["0"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"],"Connection":["close"]},"statusCode":200,"statusMessage":"OK","responseHeaders":{"content-type":"application/json","transfer-encoding":"chunked","connection":"close","cache-control":"no-store","content-encoding":"gzip","strict-transport-security":"max-age=31536000; includeSubDomains","vary":"Accept-Encoding, User-Agent","pragma":"no-cache"},"responseBody":"{\"access_token\":\"eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.vqb33-7FqzFWPNlr0ElW1v2RjJRZBel3CdDHBWD7y_o\"}","duration":200
}
    }

}"#;

pub const TEST_JSON_11: &str = r#"{
    "type": "trace",


    "data": {
"id":"11","type":"HttpRequest","timestamp":1694948911168,"http": {

      "state": "received",
"httpVersion":"1.1","method":"POST","host":"localhost","port":3000,"path":"/api/graphql","url":"http://localhost:3000/api/graphql","requestHeaders":{"authorization":"Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.vqb33-7FqzFWPNlr0ElW1v2RjJRZBel3CdDHBWD7y_o","content-type":["application/json"],"Accept":["*/*"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"]},"requestBody":"{\"query\":\"query People {\\n  people {\\n    id\\nfirstName\\nlastName\\n  }\\n}\\n\",\"operationName\":\"People\",\"variables\":{}}","statusCode":200,"statusMessage":"OK","responseHeaders":{"x-powered-by":"Express","cache-control":"private, no-store","surrogate-key":"all","access-control-allow-origin":"*","access-control-allow-credentials":"true","content-type":"application/json","content-length":"28","vary":"Accept-Encoding","date":"Thu, 17 Mar 2022 19:51:01 GMT","connection":"keep-alive","keep-alive":"timeout=5"},"responseBody":"{\"data\":{\"people\":[{\"id\":\"1\",\"firstName\":\"Peter\",\"lastName\":\"Piper\"},{\"id\":\"2\",\"firstName\":\"Tom\",\"lastName\":\"Thumbdddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd\"},{\"id\":\"3\",\"firstName\":\"Mary\",\"lastName\":\"Hadalittlelamb\"}]}}","duration":500
}    }

}"#;

pub const TEST_JSON_12: &str = r#"{

    "type": "trace",

    "data": {
"id":"12","type":"HttpRequest","timestamp":1694943912369, "http": {

      "state": "received",
"httpVersion":"1.1","method":"GET","host":"data.restserver.com","port":443,"path":"/features","url":"http://data.restserver.com/features","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"},"statusCode":200,"statusMessage":"OK","responseHeaders":{"content-type":"application/json; charset=utf-8","content-length":"351","date":"Thu, 17 Mar 2022 19:51:00 GMT","vary":"Origin","connection":"close"},"responseBody":"{\"awesomeFeature\":true,\"crappyFeature\":false}","duration":15

}    }


}"#;
//

pub const TEST_JSON_13: &str = r#"{ 
    "type": "trace",

"data": {
"id":"13","type":"HttpRequest","timestamp":1694947915469,

"http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"data.restserver.com","port":443,"path":"/countries?start=0&count=20","url":"http://data.restserver.com/countries?start=0&count=20","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"},"statusCode":404,"statusMessage":"Not found","responseHeaders":{"content-type":"application/json; charset=utf-8","content-length":"2","date":"Thu, 17 Mar 2022 19:51:01 GMT","vary":"Origin","connection":"close"},"duration":10

}

}

}"#;

pub const TEST_JSON_14: &str = r#"{

    "type": "trace",
    "data": {
"id":"14","type":"HttpRequest","timestamp":1394948931769, "http" :{

      "state": "received",
"httpVersion":"1.1","method":"POST","host":"data.restserver.com","port":443,"path":"/people","url":"http://data.restserver.com/people","requestHeaders":{"authorization":"Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.vqb33-7FqzFWPNlr0ElW1v2RjJRZBel3CdDHBWD7y_o","content-type":["application/json"],"Accept":["*/*"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"]},"requestBody":"{\"firstName\":\"Paddington\",\"lastName\":\"Bear\"}","statusCode":200,"statusMessage":"OK","responseHeaders":{"cache-control":"private, no-store","surrogate-key":"all","access-control-allow-origin":"*","access-control-allow-credentials":"true","content-type":"application/json","content-length":"11","vary":"Accept-Encoding","date":"Thu, 17 Mar 2022 19:51:02 GMT","connection":"keep-alive","keep-alive":"timeout=5"},"responseBody":"{\"id\":\"4\"}","duration":1300

}    }

}"#;

pub const TEST_JSON_15: &str = r#"{

    "type": "trace",
    "data": {
"id":"15","type":"HttpRequest","timestamp":1694448931869,"http": {

      "state": "received",
"httpVersion":"1.1","method":"POST","host":"localhost","port":3000,"path":"/api/graphql","url":"http://localhost:3000/api/graphql","requestHeaders":{"authorization":"Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.vqb33-7FqzFWPNlr0ElW1v2RjJRZBel3CdDHBWD7y_o","content-type":["application/json"],"Accept":["*/*"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"]},"requestBody":"{\"query\":\"mutation RegisterPerson($id: String) {\\n  registerPerson(id: $id) {\\n    success\\n}\\n}\",\"type\":\"HttpRequest\",\"operationName\":\"RegisterPerson\",\"variables\":{\"id\":\"4\",\"type\":\"HttpRequest\"}}","statusCode":200,"statusMessage":"OK","responseHeaders":{"x-powered-by":"Express","cache-control":"private, no-store","surrogate-key":"all","access-control-allow-origin":"*","access-control-allow-credentials":"true","content-type":"application/json","content-length":"28","vary":"Accept-Encoding","date":"Thu, 17 Mar 2022 19:51:01 GMT","connection":"keep-alive","keep-alive":"timeout=5"},"responseBody":"{\"data\":{\"success\":true}}","duration":629
}
    }
}"#;

pub const TEST_JSON_16: &str = r#"{



    "type": "trace",
    "data": {
"id":"16","type":"HttpRequest","timestamp":1694948935009, "http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"data.restserver.com","port":433,"path":"/movies?start=0&count=20","url":"https://data.restserver.com:433/movies?start=0&count=20","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"},"statusCode":500,"statusMessage":"Internal Server Error","responseHeaders":{"content-type":"application/json; charset=utf-8","content-length":"0","date":"Thu, 17 Mar 2022 19:51:01 GMT","vary":"Origin","connection":"close"},"duration":5000

}    }

}"#;

pub const TEST_JSON_17: &str = r#"{

    "type": "trace",

    "data": {
"id":"17","type":"HttpRequest","timestamp":1624948938149,"http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"hits.webstats.com","port":433,"path":"/?apikey=c82e66bd-4d5b-4bb7-b439-896936c94eb2","url":"https://hits.webstats.com:433/?apikey=c82e66bd-4d5b-4bb7-b439-896936c94eb2","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"},"statusCode":200,"statusMessage":"OK","responseHeaders":{"content-type":"application/xml; charset=utf-8","content-length":"55","date":"Thu, 17 Mar 2022 19:51:01 GMT","vary":"Origin","connection":"close"},"responseBody":"<hits><today>10</today><yesterday>15</yesterday></hits>","duration":5000

}    }

}"#;

pub const TEST_JSON_18: &str = r#"{
    "type": "trace",
    "data": {
"id":"18","type":"HttpRequest","timestamp":1694948938539,"http": {
      "state": "received",
"httpVersion":"1.1","method":"GET","host":"data.restserver.com","port":433,"path":"/features","url":"https://data.restserver.com:433/features","requestHeaders":{"accept":"application/json","User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"accept-encoding":"br, gzip, deflate"}
}
    }

}"#;

pub const TEST_JSON_19: &str = r#"{
    "type": "trace",
"data": {
"id":"1","type":"HttpRequest","timestamp":1694891653602,"http": { "timings": {
      "blocked": 1.701791,
      "dns": 37.977375,
      "connect": 38.259209,
      "state": "received",
      "send": 0.03825,
      "wait": 50.718333,
      "receive": 1.474667,
      "ssl": 21.786959
}, "duration":200,

      "state": "received",
"httpVersion":"1.1","method":"GET","host":"testserver.com","port":443,"path":"/auth?client=mock_client","url":"http://testserver.com?client=mock_client","requestHeaders":{"Authorization":["Basic dXNlcm5hbWU6cGFzc3dvcmQ="],"Content-Type":["application/x-www-form-urlencoded"],"Accept":["*/*"],"Content-Length":["0"],"User-Agent":["node-fetch/1.0 (+https://github.com/bitinn/node-fetch)"],"Accept-Encoding":["gzip,deflate"],"Connection":["close"]},"statusCode":200,"statusMessage":"OK","responseHeaders":{"content-type":"application/json","transfer-encoding":"chunked","connection":"close","cache-control":"no-store","content-encoding":"gzip","strict-transport-security":"max-age=31536000; includeSubDomains","vary":"Accept-Encoding, User-Agent","pragma":"no-cache"},"responseBody":"{\"name\":\"Juan J Hartley\",\"empty\":null,\"boolean_a\":true,\"boolean_b\":false,\"phones\":[\"+44 1234567\",\"+44 2345678\"],\"age\":43,\"nested\":{\"name\":\"Imogene Thompson\",\"nested\":{\"name\":\"Sandy Feldstein\"}}}"
}
} }"#;
