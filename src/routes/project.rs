/*
 * Here's how project works:
 * 1. User create a project
 * 2. User will add all the necessary information such as roles, members, areas, task
 * 3. User will ada a timeline for each tasks
 * 4. User can start the project by creating a day 0 report, which is a daily report that only have a plan about the next day's work
 * 5. Project's status will change to "running" if at least one task's status is changed to running
 * 6. Project's status will change to "paused" if once a project runs and there's no tasks that has a "running" status
 * 7. Project's status will chang eto "finished" if all tasks have a status of "finished"
 * 8. Each tasks progress is calculated by every progress report's actual progress
 * 9.
 */

use std::{str::FromStr, vec};

use actix_web::{get, patch, post, web, HttpMessage, HttpRequest, HttpResponse};
use chrono::Utc;
use mongodb::bson::{oid::ObjectId, DateTime};

use crate::models::{
    project::{
        Project, ProjectArea, ProjectMember, ProjectMemberKind, ProjectPeriod, ProjectRequest,
        ProjectStatus, ProjectStatusKind,
    },
    project_progress_report::{ProjectProgressReport, ProjectProgressReportRequest},
    project_role::{ProjectRole, ProjectRolePermission, ProjectRoleRequest},
    project_task::{
        ProjectTask, ProjectTaskPeriodRequest, ProjectTaskRequest, ProjectTaskStatus,
        ProjectTaskStatusRequest,
    },
    role::{Role, RolePermission},
    user::UserAuthentication,
};

// #[get("/projects")]
// pub async fn get_projects() -> HttpResponse {
//     let query: ProjectQuery = ProjectQuery {
//         _id: None,
//         limit: None,
//     };

//     match Project::find_many(&query).await {
//         Ok(projects) => HttpResponse::Ok().json(projects),
//         Err(error) => HttpResponse::BadRequest().body(error),
//     }
// }
#[get("/projects/{_id}")]
pub async fn get_project(_id: web::Path<String>) -> HttpResponse {
    let _id: String = _id.into_inner();
    if let Ok(_id) = ObjectId::from_str(&_id) {
        return match Project::find_by_id(&_id).await {
            Ok(Some(project)) => HttpResponse::Ok().json(project),
            Ok(None) => HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::BadRequest().body("INVALID_ID".to_string())
    }
}
// #[delete("/projects/{_id}")]
// pub async fn delete_project(_id: web::Path<String>) -> HttpResponse {
//     let _id: String = _id.into_inner();
//     if let Ok(_id) = ObjectId::from_str(&_id) {
//         return match Project::delete_by_id(&_id).await {
//             Ok(count) => HttpResponse::Ok().body(format!("Deleted {count} project")),
//             Err(error) => HttpResponse::InternalServerError().body(error),
//         };
//     } else {
//         HttpResponse::BadRequest().body("INVALID_ID".to_string())
//     }
// }
#[post("/projects")]
pub async fn create_project(payload: web::Json<ProjectRequest>, req: HttpRequest) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        if !Role::validate(&issuer.role, &RolePermission::CreateProject).await {
            return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
        }

        let payload: ProjectRequest = payload.into_inner();

        let project_id = Some(ObjectId::new()).unwrap();

        let mut project_role: ProjectRole = ProjectRole {
            _id: None,
            name: "Owner".to_string(),
            permission: vec![ProjectRolePermission::Owner],
            project_id,
        };

        match project_role.save().await {
            Ok(role_id) => {
                let member: ProjectMember = ProjectMember {
                    _id: issuer._id.unwrap(),
                    role_id: vec![role_id],
                    kind: ProjectMemberKind::Indirect,
                    name: None,
                };

                let project: Project = Project {
                    _id: Some(project_id),
                    customer_id: payload.customer_id,
                    name: payload.name,
                    code: payload.code,
                    status: vec![ProjectStatus {
                        kind: ProjectStatusKind::Pending,
                        time: DateTime::from_millis(Utc::now().timestamp_millis()),
                        message: None,
                    }],
                    member: Some(vec![member]),
                    area: None,
                    holiday: payload.holiday,
                };
                match project.save().await {
                    Ok(_) => {
                        HttpResponse::Ok().body(project_id.to_string())
                        // @TODO: Add preset!
                    }
                    Err(error) => {
                        ProjectRole::delete_by_id(&role_id)
                            .await
                            .expect("ROLE_DELETION_FAILED");
                        HttpResponse::InternalServerError().body(error)
                    }
                }
            }
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}
#[post("/projects/{_id}/roles")]
pub async fn create_project_role(
    _id: web::Path<String>,
    payload: web::Json<ProjectRoleRequest>,
    req: HttpRequest,
) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        let _id = _id.into_inner();
        if let Ok(_id) = ObjectId::from_str(&_id) {
            if !ProjectRole::validate(
                &_id,
                &issuer._id.unwrap(),
                &ProjectRolePermission::CreateRole,
            )
            .await
            {
                return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
            }

            let payload: ProjectRoleRequest = payload.into_inner();

            let mut project_role: ProjectRole = ProjectRole {
                _id: None,
                project_id: _id,
                name: payload.name,
                permission: payload.permission,
            };

            match project_role.save().await {
                Ok(role_id) => HttpResponse::Ok().body(role_id.to_string()),
                Err(error) => HttpResponse::InternalServerError().body(error),
            }
        } else {
            HttpResponse::BadRequest().body("INVALID_ID".to_string())
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}
#[post("/projects/{_id}/tasks")]
pub async fn create_project_task(
    _id: web::Path<String>,
    payload: web::Json<ProjectTaskRequest>,
    req: HttpRequest,
) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        let _id: String = _id.into_inner();
        if let Ok(_id) = ObjectId::from_str(&_id) {
            if !ProjectRole::validate(
                &_id,
                &issuer._id.unwrap(),
                &ProjectRolePermission::CreateRole,
            )
            .await
            {
                return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
            }

            let payload: ProjectTaskRequest = payload.into_inner();

            let mut project_task: ProjectTask = ProjectTask {
                _id: None,
                project_id: _id,
                area_id: payload.area_id,
                name: payload.name,
                volume: payload.volume,
                period: None,
                status: vec![ProjectTaskStatus {
                    kind: crate::models::project_task::ProjectTaskStatusKind::Pending,
                    time: DateTime::from_millis(Utc::now().timestamp_millis()),
                    message: None,
                }],
            };

            match project_task.save().await {
                Ok(task_id) => HttpResponse::Ok().body(task_id.to_string()),
                Err(error) => HttpResponse::InternalServerError().body(error),
            }
        } else {
            HttpResponse::BadRequest().body("INVALID_ID".to_string())
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}
#[post("/projects/{_id}/reports")]
pub async fn create_project_report(
    _id: web::Path<String>,
    payload: web::Json<ProjectProgressReportRequest>,
    req: HttpRequest,
) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        let _id: String = _id.into_inner();
        if let Ok(_id) = ObjectId::from_str(&_id) {
            if !ProjectRole::validate(
                &_id,
                &issuer._id.unwrap(),
                &ProjectRolePermission::CreateRole,
            )
            .await
            {
                return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
            }

            let payload: ProjectProgressReportRequest = payload.into_inner();

            let mut project_report: ProjectProgressReport = ProjectProgressReport {
                _id: None,
                project_id: _id,
                date: DateTime::from_millis(Utc::now().timestamp_millis()),
                time: payload.time,
                actual: payload.actual,
                plan: payload.plan,
                documentation: None,
                weather: payload.weather,
            };

            match project_report.save().await {
                Ok(report_id) => HttpResponse::Ok().body(report_id.to_string()),
                Err(error) => HttpResponse::InternalServerError().body(error),
            }
        } else {
            HttpResponse::BadRequest().body("INVALID_ID".to_string())
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}

#[patch("/projects/{_id}/tasks/status")]
pub async fn update_project_task_status(
    _id: web::Path<String>,
    payload: web::Json<ProjectTaskStatusRequest>,
    req: HttpRequest,
) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        let _id: String = _id.into_inner();
        if let Ok(_id) = ObjectId::from_str(&_id) {
            if !ProjectRole::validate(
                &_id,
                &issuer._id.unwrap(),
                &ProjectRolePermission::CreateRole,
            )
            .await
            {
                return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
            }

            if let Ok(Some(mut task)) = ProjectTask::find_by_id(&_id).await {
                let payload: ProjectTaskStatusRequest = payload.into_inner();

                match task.update_status(payload.kind, payload.message).await {
                    Ok(task_id) => HttpResponse::Ok().body(task_id.to_string()),
                    Err(error) => HttpResponse::InternalServerError().body(error),
                }
            } else {
                HttpResponse::NotFound().body("PROJECT_TASK_NOT_FOUND".to_string())
            }
        } else {
            HttpResponse::BadRequest().body("INVALID_ID".to_string())
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}
#[patch("/projects/{_id}/tasks/period")]
pub async fn update_project_task_period(
    _id: web::Path<String>,
    payload: web::Json<ProjectTaskPeriodRequest>,
    req: HttpRequest,
) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        let _id: String = _id.into_inner();
        if let Ok(_id) = ObjectId::from_str(&_id) {
            if !ProjectRole::validate(
                &_id,
                &issuer._id.unwrap(),
                &ProjectRolePermission::CreateRole,
            )
            .await
            {
                return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
            }

            if let Ok(Some(mut task)) = ProjectTask::find_by_id(&_id).await {
                let payload: ProjectTaskPeriodRequest = payload.into_inner();

                let period: ProjectPeriod = ProjectPeriod {
                    start: payload.start,
                    end: payload.end,
                };

                match task.update_period(period).await {
                    Ok(task_id) => HttpResponse::Ok().body(task_id.to_string()),
                    Err(error) => HttpResponse::InternalServerError().body(error),
                }
            } else {
                HttpResponse::NotFound().body("PROJECT_TASK_NOT_FOUND".to_string())
            }
        } else {
            HttpResponse::BadRequest().body("INVALID_ID".to_string())
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}
#[patch("/projects/{_id}/members")]
pub async fn add_project_member(
    _id: web::Path<String>,
    payload: web::Json<ProjectMember>,
    req: HttpRequest,
) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        let _id: String = _id.into_inner();
        if let Ok(_id) = ObjectId::from_str(&_id) {
            if !ProjectRole::validate(
                &_id,
                &issuer._id.unwrap(),
                &ProjectRolePermission::CreateRole,
            )
            .await
            {
                return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
            }

            if let Ok(Some(mut project)) = Project::find_by_id(&_id).await {
                let payload: ProjectMember = payload.into_inner();

                match project.add_member(&vec![payload]).await {
                    Ok(project_id) => HttpResponse::Ok().body(project_id.to_string()),
                    Err(error) => HttpResponse::InternalServerError().body(error),
                }
            } else {
                HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
            }
        } else {
            HttpResponse::BadRequest().body("INVALID_ID".to_string())
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}
#[patch("/projects/{_id}/areas")]
pub async fn add_project_area(
    _id: web::Path<String>,
    payload: web::Json<ProjectArea>,
    req: HttpRequest,
) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        let _id: String = _id.into_inner();
        if let Ok(_id) = ObjectId::from_str(&_id) {
            if !ProjectRole::validate(
                &_id,
                &issuer._id.unwrap(),
                &ProjectRolePermission::CreateRole,
            )
            .await
            {
                return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
            }

            if let Ok(Some(mut project)) = Project::find_by_id(&_id).await {
                let payload: ProjectArea = payload.into_inner();

                match project.add_area(&vec![payload]).await {
                    Ok(project_id) => HttpResponse::Ok().body(project_id.to_string()),
                    Err(error) => HttpResponse::InternalServerError().body(error),
                }
            } else {
                HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
            }
        } else {
            HttpResponse::BadRequest().body("INVALID_ID".to_string())
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}
