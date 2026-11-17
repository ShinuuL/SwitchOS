// SwitchOS: friend:u / friend:a - a console with no friends and no notifications. Pokémon Legends: Arceus asks for
// the friend list while it starts (nn::friends::GetFriendList): the logging stub returned no IFriendService, and the
// SDK's call through that null object crashed (exception c0000005 at f_70081a4280, Arceus 1.0.1). Every list and count
// is empty (outputs zeroed: count 0), the events never signal, Pop has nothing to give.
#include <stdio.h>
#include <string.h>

#include "../hle.h"
#include "../ipc.h"

// INotificationService::Pop with nothing queued (what a console answers: account module, 15)
#define FRIENDS_NO_NOTIFICATION MAKE_RESULT(124, 15)

static uint32_t new_event_handle(void) {
    uint32_t w, r;
    kernel_create_event_pair(&w, &r);
    return r;
}

static void friend_service(Service *s, IpcReq *req, IpcResp *resp) {
    (void)s;
    switch (req->cmd) {
    case 0: resp->copy_handles[resp->ncopy++] = new_event_handle(); break;  // GetCompletionEvent
    // GetFriendListIds / GetFriendList / GetBlockedUserListIds / GetFriendCount / GetNewlyFriendCount...: count 0
    default:
        LOG("[friends] IFriendService cmd %u (empty)\n", req->cmd);
        resp_size(resp, 0x10);
    }
}

static void friend_notification(Service *s, IpcReq *req, IpcResp *resp) {
    (void)s;
    switch (req->cmd) {
    case 0: resp->copy_handles[resp->ncopy++] = new_event_handle(); break;  // GetEvent
    case 2: resp->result = FRIENDS_NO_NOTIFICATION; break;                    // Pop
    default: resp_size(resp, 0x10);                                         // Clear...
    }
}

static void friend_root(Service *s, IpcReq *req, IpcResp *resp) {
    (void)s;
    if (req->cmd == 0) ipc_return_object(resp, service_new("IFriendService", friend_service, NULL));
    else if (req->cmd == 1) ipc_return_object(resp, service_new("INotificationService", friend_notification, NULL));
    else { LOG("[friends] IServiceCreator cmd %u (ok)\n", req->cmd); resp_size(resp, 0x10); }
}
SERVICE("friend:u", friend_u) { friend_root(s, req, resp); }
SERVICE("friend:a", friend_a) { friend_root(s, req, resp); }
