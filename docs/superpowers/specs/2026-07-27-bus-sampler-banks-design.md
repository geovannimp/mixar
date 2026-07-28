# Bus sampler bank CRUD (host-handled)

Date: 2026-07-27  
Parent: [engine event bus](./2026-07-26-engine-event-bus-design.md)  
Status: accepted

## Goal

`create_sampler_bank`, `update_sampler_bank`, `delete_sampler_bank` via `publishCmd` / host-handled `engine_publish`.

| Kind | Origin | Body |
|------|--------|------|
| `create_sampler_bank` | Deck | `{ name?, play_mode? }` |
| `update_sampler_bank` | Mixer | `{ bank_id, name, play_mode? }` |
| `delete_sampler_bank` | Mixer | `{ bank_id }` |

Not forwarded to omnibus. `list_sampler_banks` / `get_sampler_status` stay query invokes.
