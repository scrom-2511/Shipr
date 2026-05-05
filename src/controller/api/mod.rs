pub mod github;
pub mod vm_request_proxy;

// {
//   "action": "created",
//   "installation": {
//     "id": 128645812,
//     "client_id": "Iv23li2CQJsPH0rjouV1",
//     "account": {
//       "login": "scrom-2511",
//       "id": 195581508,
//       "node_id": "U_kgDOC6hWRA",
//       "avatar_url": "https://avatars.githubusercontent.com/u/195581508?v=4",
//       "gravatar_id": "",
//       "url": "https://api.github.com/users/scrom-2511",
//       "html_url": "https://github.com/scrom-2511",
//       "followers_url": "https://api.github.com/users/scrom-2511/followers",
//       "following_url": "https://api.github.com/users/scrom-2511/following{/other_user}",
//       "gists_url": "https://api.github.com/users/scrom-2511/gists{/gist_id}",
//       "starred_url": "https://api.github.com/users/scrom-2511/starred{/owner}{/repo}",
//       "subscriptions_url": "https://api.github.com/users/scrom-2511/subscriptions",
//       "organizations_url": "https://api.github.com/users/scrom-2511/orgs",
//       "repos_url": "https://api.github.com/users/scrom-2511/repos",
//       "events_url": "https://api.github.com/users/scrom-2511/events{/privacy}",
//       "received_events_url": "https://api.github.com/users/scrom-2511/received_events",
//       "type": "User",
//       "user_view_type": "public",
//       "site_admin": false
//     },
//     "repository_selection": "selected",
//     "access_tokens_url": "https://api.github.com/app/installations/128645812/access_tokens",
//     "repositories_url": "https://api.github.com/installation/repositories",
//     "html_url": "https://github.com/settings/installations/128645812",
//     "app_id": 3566236,
//     "app_slug": "shipr-deployment",
//     "target_id": 195581508,
//     "target_type": "User",
//     "permissions": {
//       "contents": "read",
//       "metadata": "read"
//     },
//     "events": [
//       "push"
//     ],
//     "created_at": "2026-05-01T17:45:04.000+05:30",
//     "updated_at": "2026-05-01T17:45:05.000+05:30",
//     "single_file_name": null,
//     "has_multiple_single_files": false,
//     "single_file_paths": [],
//     "suspended_by": null,
//     "suspended_at": null
//   },
//   "repositories": [
//     {
//       "id": 1201293385,
//       "node_id": "R_kgDOR5pISQ",
//       "name": "shipr_test_project",
//       "full_name": "scrom-2511/shipr_test_project",
//       "private": false
//     }
//   ],
//   "requester": null,
//   "sender": {
//     "login": "scrom-2511",
//     "id": 195581508,
//     "node_id": "U_kgDOC6hWRA",
//     "avatar_url": "https://avatars.githubusercontent.com/u/195581508?v=4",
//     "gravatar_id": "",
//     "url": "https://api.github.com/users/scrom-2511",
//     "html_url": "https://github.com/scrom-2511",
//     "followers_url": "https://api.github.com/users/scrom-2511/followers",
//     "following_url": "https://api.github.com/users/scrom-2511/following{/other_user}",
//     "gists_url": "https://api.github.com/users/scrom-2511/gists{/gist_id}",
//     "starred_url": "https://api.github.com/users/scrom-2511/starred{/owner}{/repo}",
//     "subscriptions_url": "https://api.github.com/users/scrom-2511/subscriptions",
//     "organizations_url": "https://api.github.com/users/scrom-2511/orgs",
//     "repos_url": "https://api.github.com/users/scrom-2511/repos",
//     "events_url": "https://api.github.com/users/scrom-2511/events{/privacy}",
//     "received_events_url": "https://api.github.com/users/scrom-2511/received_events",
//     "type": "User",
//     "user_view_type": "public",
//     "site_admin": false
//   }
// }

// let project_id = uuid::Uuid::new_v4();

// let presigned_upload_url = s3_service
//     .get_presigned_upload_url(&project_id.to_string())
//     .await?;

// for _ in 0..1 {
//     let new_id = id_allocator.allocate_id().await? as u32;
//     let mut new_vm = Firecracker::new(new_id);

//     new_vm.create_vm().await?;
//     vm_pool.add_to_ideal_vms(new_id);
// }

// let deploy_details = DeployDetails {
//     url,
//     install_commands: install,
//     build_commands: build,
//     branch,
//     project_id,
//     home_dir,
//     dist_dir,
//     presigned_upload_url,
// };

// let mut job_dispatcher = JobDispatcher::new(vm_pool, s3_service);
// job_dispatcher.dispatch_deploy_job(&deploy_details).await?;

// {
//   "ref": "refs/heads/main",
//   "before": "08133af8850d3c68a75c8d29c622425a780151d2",
//   "after": "cd9671b3cd49b1a99824c7e1aaeda6e853a3bd38",
//   "repository": {
//     "id": 1201293385,
//     "node_id": "R_kgDOR5pISQ",
//     "name": "shipr_test_project",
//     "full_name": "scrom-2511/shipr_test_project",
//     "private": false,
//     "owner": {
//       "name": "scrom-2511",
//       "email": "formain337@gmail.com",
//       "login": "scrom-2511",
//       "id": 195581508,
//       "node_id": "U_kgDOC6hWRA",
//       "avatar_url": "https://avatars.githubusercontent.com/u/195581508?v=4",
//       "gravatar_id": "",
//       "url": "https://api.github.com/users/scrom-2511",
//       "html_url": "https://github.com/scrom-2511",
//       "followers_url": "https://api.github.com/users/scrom-2511/followers",
//       "following_url": "https://api.github.com/users/scrom-2511/following{/other_user}",
//       "gists_url": "https://api.github.com/users/scrom-2511/gists{/gist_id}",
//       "starred_url": "https://api.github.com/users/scrom-2511/starred{/owner}{/repo}",
//       "subscriptions_url": "https://api.github.com/users/scrom-2511/subscriptions",
//       "organizations_url": "https://api.github.com/users/scrom-2511/orgs",
//       "repos_url": "https://api.github.com/users/scrom-2511/repos",
//       "events_url": "https://api.github.com/users/scrom-2511/events{/privacy}",
//       "received_events_url": "https://api.github.com/users/scrom-2511/received_events",
//       "type": "User",
//       "user_view_type": "public",
//       "site_admin": false
//     },
//     "html_url": "https://github.com/scrom-2511/shipr_test_project",
//     "description": null,
//     "fork": false,
//     "url": "https://api.github.com/repos/scrom-2511/shipr_test_project",
//     "created_at": 1775309580,
//     "updated_at": "2026-05-02T12:27:57Z",
//     "pushed_at": 1777725936,
//     "git_url": "git://github.com/scrom-2511/shipr_test_project.git",
//     "ssh_url": "git@github.com:scrom-2511/shipr_test_project.git",
//     "clone_url": "https://github.com/scrom-2511/shipr_test_project.git",
//     "svn_url": "https://github.com/scrom-2511/shipr_test_project",
//     "size": 119,
//     "stargazers_count": 0,
//     "watchers_count": 0,
//     "language": "TypeScript",
//     "has_issues": true,
//     "has_projects": true,
//     "has_downloads": true,
//     "has_wiki": true,
//     "has_pages": false,
//     "has_discussions": false,
//     "forks_count": 0,
//     "archived": false,
//     "disabled": false,
//     "open_issues_count": 0,
//     "allow_forking": true,
//     "is_template": false,
//     "web_commit_signoff_required": false,
//     "has_pull_requests": true,
//     "pull_request_creation_policy": "all",
//     "visibility": "public",
//     "default_branch": "main"
//   },
//   "pusher": {
//     "name": "scrom-2511",
//     "email": "formain337@gmail.com"
//   },
//   "forced": false,
//   "created": false,
//   "deleted": false,
//   "compare": "https://github.com/scrom-2511/shipr_test_project/compare/08133af8850d...cd9671b3cd49",
//   "commits": [
//     {
//       "id": "cd9671b3cd49b1a99824c7e1aaeda6e853a3bd38",
//       "message": "Update App.tsx\n\nthis is a new test update 2",
//       "timestamp": "2026-05-02T18:15:35+05:30",
//       "url": "https://github.com/scrom-2511/shipr_test_project/commit/cd9671b3cd49b1a99824c7e1aaeda6e853a3bd38",
//       "author": {
//         "name": "Scrom",
//         "email": "formain337@gmail.com",
//         "username": "scrom-2511"
//       },
//       "committer": {
//         "name": "GitHub",
//         "email": "noreply@github.com",
//         "username": "web-flow"
//       },
//       "modified": [
//         "frontend/src/App.tsx"
//       ]
//     }
//   ],
//   "head_commit": {
//     "id": "cd9671b3cd49b1a99824c7e1aaeda6e853a3bd38",
//     "message": "Update App.tsx\n\nthis is a new test update 2",
//     "timestamp": "2026-05-02T18:15:35+05:30",
//     "url": "https://github.com/scrom-2511/shipr_test_project/commit/cd9671b3cd49b1a99824c7e1aaeda6e853a3bd38",
//     "author": {
//       "name": "Scrom",
//       "email": "formain337@gmail.com",
//       "username": "scrom-2511"
//     },
//     "committer": {
//       "name": "GitHub",
//       "email": "noreply@github.com",
//       "username": "web-flow"
//     },
//     "modified": [
//       "frontend/src/App.tsx"
//     ]
//   }
// }
