window.SIDEBAR_ITEMS = {"fn":[["accept4_detour",""],["accept_detour",""],["bind_detour",""],["connect_detour",""],["dup2_detour",""],["dup3_detour",""],["dup_detour",""],["enable_socket_hooks",""],["fcntl_detour","https://github.com/metalbear-co/mirrord/issues/184"],["freeaddrinfo_detour","Deallocates a `*mut libc::addrinfo` that was previously allocated with `Box::new` in `getaddrinfo_detour` and converted into a raw pointer by `Box::into_raw`. Same thing must also be done for `addrinfo.ai_addr`."],["getaddrinfo_detour","Turns the raw pointer parameters into Rust types and calls `ops::getaddrinfo`."],["gethostname_detour","Hook for `libc::gethostname`."],["getpeername_detour",""],["getsockname_detour",""],["listen_detour",""],["socket_detour",""],["uv__accept4_detour",""]],"static":[["FN_ACCEPT",""],["FN_ACCEPT4",""],["FN_BIND",""],["FN_CONNECT",""],["FN_DUP",""],["FN_DUP2",""],["FN_DUP3",""],["FN_FCNTL",""],["FN_FREEADDRINFO",""],["FN_GETADDRINFO",""],["FN_GETHOSTNAME",""],["FN_GETPEERNAME",""],["FN_GETSOCKNAME",""],["FN_LISTEN",""],["FN_SOCKET",""],["FN_UV__ACCEPT4",""],["MANAGED_ADDRINFO","Here we keep addr infos that we allocated so we’ll know when to use the original freeaddrinfo function and when to use our implementation"]],"type":[["FnAccept",""],["FnAccept4",""],["FnBind",""],["FnConnect",""],["FnDup",""],["FnDup2",""],["FnDup3",""],["FnFcntl",""],["FnFreeaddrinfo",""],["FnGetaddrinfo",""],["FnGethostname",""],["FnGetpeername",""],["FnGetsockname",""],["FnListen",""],["FnSocket",""],["FnUv__accept4",""]]};