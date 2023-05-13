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

use std::{
    ffi::OsStr,
    fs::{create_dir_all, remove_dir_all, rename},
    path::{Path, PathBuf},
    vec,
};

use actix_multipart::form::MultipartForm;
use actix_web::{get, patch, post, put, web, HttpMessage, HttpRequest, HttpResponse};
use chrono::Utc;
use mongodb::bson::{doc, oid::ObjectId, to_bson, DateTime};

use crate::models::{
    project::{
        Project, ProjectAreaRequest, ProjectMember, ProjectMemberKind, ProjectRequest,
        ProjectStatus, ProjectStatusKind,
    },
    project_incident_report::{
        ProjectIncidentReport, ProjectIncidentReportRequest, ProjectIncidentReportRequestQuery,
    },
    project_progress_report::{
        ProjectProgressReport, ProjectProgressReportDocumentationRequest,
        ProjectProgressReportRequest,
    },
    project_role::{ProjectRole, ProjectRolePermission, ProjectRoleRequest},
    project_task::{
        ProjectTask, ProjectTaskPeriod, ProjectTaskPeriodRequest, ProjectTaskRequest,
        ProjectTaskStatus, ProjectTaskStatusKind, ProjectTaskStatusRequest,
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

#[get("/projects/{project_id}")]
pub async fn get_project(project_id: web::Path<String>) -> HttpResponse {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    match Project::find_by_id(&project_id).await {
        Ok(Some(project)) => HttpResponse::Ok().json(project),
        Ok(None) => HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/projects")] // FINISHED
pub async fn create_project(payload: web::Json<ProjectRequest>, req: HttpRequest) -> HttpResponse {
    let issuer = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if issuer.role.is_empty() || !Role::validate(&issuer.role, &RolePermission::CreateProject).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let payload: ProjectRequest = payload.into_inner();

    let mut project: Project = Project {
        _id: None,
        customer_id: payload.customer_id,
        name: payload.name,
        code: payload.code,
        status: vec![ProjectStatus {
            kind: ProjectStatusKind::Pending,
            time: DateTime::from_millis(Utc::now().timestamp_millis()),
            message: None,
        }],
        member: None,
        area: None,
        holiday: payload.holiday,
    };
    match project.save().await {
        Ok(project_id) => {
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

                    match project.add_member(&[member]).await {
                        Ok(project_id) => HttpResponse::Ok().body(project_id.to_string()),
                        Err(error) => {
                            Project::delete_by_id(&project_id)
                                .await
                                .expect("PROJECT_DELETION_FAILED");
                            ProjectRole::delete_by_id(&role_id)
                                .await
                                .expect("PROJECT_ROLE_DELETION_FAILED");
                            HttpResponse::InternalServerError().body(error)
                        }
                    }
                }
                Err(error) => {
                    Project::delete_by_id(&project_id)
                        .await
                        .expect("PROJECT_DELETION_FAILED");
                    HttpResponse::InternalServerError().body(error)
                }
            }
            // @TODO: Add preset!
        }
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/projects/{project_id}/roles")] // FINISHED
pub async fn create_project_role(
    project_id: web::Path<String>,
    payload: web::Json<ProjectRoleRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::CreateRole).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let payload: ProjectRoleRequest = payload.into_inner();

    let mut project_role: ProjectRole = ProjectRole {
        _id: None,
        project_id,
        name: payload.name,
        permission: payload.permission,
    };

    match project_role.save().await {
        Ok(role_id) => HttpResponse::Ok().body(role_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}

#[post("/projects/{project_id}/tasks")] // FINISHED
pub async fn create_project_task(
    project_id: web::Path<String>,
    payload: web::Json<ProjectTaskRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::CreateTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }
    let payload: ProjectTaskRequest = payload.into_inner();

    let mut project_task: ProjectTask = ProjectTask {
        _id: None,
        project_id,
        area_id: ObjectId::new(),
        task_id: None,
        user_id: payload.user_id,
        name: payload.name,
        volume: payload.volume,
        value: payload.value,
        description: payload.description,
        period: None,
        status: vec![ProjectTaskStatus {
            kind: ProjectTaskStatusKind::Pending,
            time: DateTime::from_millis(Utc::now().timestamp_millis()),
            message: None,
        }],
    };

    if let Some(area_id) = payload.area_id {
        project_task.area_id = area_id
    } else {
        return HttpResponse::BadRequest().body("PROJECT_TASK_MUST_HAVE_AREA_ID".to_string());
    }

    match project_task.save().await {
        Ok(task_id) => HttpResponse::Ok().body(task_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/projects/{project_id}/tasks/{task_id}")] // FINISHED
pub async fn create_project_task_sub(
    _id: web::Path<(String, String)>,
    payload: web::Json<Vec<ProjectTaskRequest>>,
    req: HttpRequest,
) -> HttpResponse {
    let (project_id, task_id) = match (_id.0.parse(), _id.1.parse()) {
        (Ok(project_id), Ok(task_id)) => (project_id, task_id),
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::CreateTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    if let Ok(Some(task)) = ProjectTask::find_by_id(&task_id).await {
        if let Ok(Some(project)) = Project::find_by_id(&task.project_id).await {
            if project.status.get(0).unwrap().kind != ProjectStatusKind::Pending {
                return HttpResponse::BadRequest()
                    .body("PROJECT_STATUS_MUST_BE_PENDING".to_string());
            }
            let payload = payload.into_inner();
            let mut new_task_id = Vec::<ObjectId>::new();
            let mut total = 0.0;

            for i in &payload {
                total += i.value;
            }

            if total != 100.0 {
                return HttpResponse::BadRequest().body("PROJECT_TASK_VALUE_SUM_MUST_BE_100");
            }

            for i in payload {
                let mut project_task: ProjectTask = ProjectTask {
                    _id: None,
                    project_id,
                    area_id: task.area_id,
                    task_id: Some(task_id),
                    user_id: i.user_id,
                    name: i.name,
                    volume: i.volume,
                    value: i.value,
                    description: i.description,
                    period: None,
                    status: vec![ProjectTaskStatus {
                        kind: ProjectTaskStatusKind::Pending,
                        time: DateTime::from_millis(Utc::now().timestamp_millis()),
                        message: None,
                    }],
                };
                match project_task.save().await {
                    Ok(task_id) => new_task_id.push(task_id),
                    Err(error) => {
                        for i in new_task_id {
                            ProjectTask::delete_by_id(&i)
                                .await
                                .expect("PROJECT_TASK_DELETION_FAILED");
                        }
                        return HttpResponse::InternalServerError().body(error);
                    }
                }
            }

            HttpResponse::Ok().json(doc! {
                "_id": to_bson::<Vec<ObjectId>>(&new_task_id).unwrap()
            })
        } else {
            HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_TASK_NOT_FOUND".to_string())
    }
}

#[post("/projects/{project_id}/reports")]
pub async fn create_project_report(
    project_id: web::Path<String>,
    payload: web::Json<ProjectProgressReportRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(
        &project_id,
        &issuer_id,
        &ProjectRolePermission::CreateReport,
    )
    .await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let payload: ProjectProgressReportRequest = payload.into_inner();

    let mut project_report = ProjectProgressReport {
        _id: None,
        project_id,
        date: DateTime::from_millis(Utc::now().timestamp_millis()),
        time: payload.time,
        actual: payload.actual,
        plan: payload.plan,
        documentation: payload.documentation,
        weather: payload.weather,
    };

    match project_report.save().await {
        Ok(report_id) => HttpResponse::Ok().body(report_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}

#[post("/projects/{project_id}/incidents")]
pub async fn create_project_incident(
    project_id: web::Path<String>,
    payload: web::Json<ProjectIncidentReportRequest>,
    query: web::Query<ProjectIncidentReportRequestQuery>,
    req: HttpRequest,
) -> HttpResponse {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(
        &project_id,
        &issuer_id,
        &ProjectRolePermission::CreateIncident,
    )
    .await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let payload: ProjectIncidentReportRequest = payload.into_inner();

    let mut project_incident = ProjectIncidentReport {
        _id: None,
        project_id,
        user_id: payload.user_id,
        kind: payload.kind,
        date: DateTime::from_millis(Utc::now().timestamp_millis()),
    };

    match project_incident.save(query.breakdown).await {
        Ok(incident_id) => HttpResponse::Ok().body(incident_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}

#[put("/projects/{project_id}/tasks/{task_id}")] // FINISHED
pub async fn update_project_task(
    _id: web::Path<(String, String)>,
    payload: web::Json<ProjectTaskRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let (project_id, task_id) = match (_id.0.parse(), _id.1.parse()) {
        (Ok(project_id), Ok(task_id)) => (project_id, task_id),
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::CreateTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    if let Ok(Some(mut task)) = ProjectTask::find_by_id(&task_id).await {
        if let Ok(Some(project)) = Project::find_by_id(&task.project_id).await {
            if project.status.get(0).unwrap().kind != ProjectStatusKind::Pending {
                return HttpResponse::BadRequest()
                    .body("PROJECT_STATUS_MUST_BE_PENDING".to_string());
            }
            let payload: ProjectTaskRequest = payload.into_inner();

            task.name = payload.name;
            task.volume = payload.volume;
            task.description = payload.description;
            task.value = payload.value;

            match task.update().await {
                Ok(task_id) => HttpResponse::Ok().body(task_id.to_string()),
                Err(error) => HttpResponse::InternalServerError().body(error),
            }
        } else {
            HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_TASK_NOT_FOUND".to_string())
    }
}

#[patch("/projects/{project_id}/tasks/{task_id}/status")]
pub async fn update_project_task_status(
    _id: web::Path<(String, String)>,
    payload: web::Json<ProjectTaskStatusRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let (project_id, task_id) = match (_id.0.parse(), _id.1.parse()) {
        (Ok(project_id), Ok(task_id)) => (project_id, task_id),
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::CreateTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    if let Ok(Some(mut task)) = ProjectTask::find_by_id(&task_id).await {
        let payload: ProjectTaskStatusRequest = payload.into_inner();

        match task.update_status(payload.kind, payload.message).await {
            Ok(task_id) => HttpResponse::Ok().body(task_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_TASK_NOT_FOUND".to_string())
    }
}
#[patch("/projects/{project_id}/tasks/{task_id}/period")]
pub async fn update_project_task_period(
    _id: web::Path<(String, String)>,
    payload: web::Json<ProjectTaskPeriodRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let (project_id, task_id) = match (_id.0.parse(), _id.1.parse()) {
        (Ok(project_id), Ok(task_id)) => (project_id, task_id),
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::CreateTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    if let Ok(Some(mut task)) = ProjectTask::find_by_id(&task_id).await {
        let payload: ProjectTaskPeriodRequest = payload.into_inner();

        let period: ProjectTaskPeriod = ProjectTaskPeriod {
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
}
#[patch("/projects/{project_id}/reports/{report_id}")] // REDO ALL CHANGES WHEN FAILED
pub async fn update_project_report(
    _id: web::Path<(String, String)>,
    form: MultipartForm<ProjectProgressReportDocumentationRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let (project_id, report_id) = match (_id.0.parse(), _id.1.parse()) {
        (Ok(project_id), Ok(report_id)) => (project_id, report_id),
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::UpdateTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let mut report = match ProjectProgressReport::find_by_id(&report_id).await {
        Ok(Some(report)) => report,
        _ => return HttpResponse::NotFound().body("PROJECT_REPORT_NOT_FOUND".to_string()),
    };

    let save_dir = format!("./files/reports/documentation/{}/", report_id);

    if create_dir_all(&save_dir).is_err() {
        return HttpResponse::InternalServerError().body("DIRECTORY_CREATION_FAILED".to_string());
    }

    let mut documentation = match report.documentation {
        Some(documentation) => {
            if documentation.len() != form.files.len() {
                ProjectProgressReport::delete_by_id(&report_id)
                    .await
                    .expect("PROJECT_REPORT_DELETION_FAILED");
                return HttpResponse::BadRequest()
                    .body("PROJECT_REPORT_DOCUMENTATION_INVALID_LENGTH".to_string());
            }
            documentation
        }
        None => {
            return HttpResponse::BadRequest()
                .body("PROJECT_REPORT_DOCUMENTATION_NOT_FOUND".to_string())
        }
    };

    for (i, file) in form.files.iter().enumerate() {
        if let Some(mut image) = documentation.get_mut(i) {
            if let Some(_id) = image._id {
                let mut ext: String = String::new();
                if let Some(file_name) = &file.file_name {
                    if let Some(name) = Path::new(file_name).extension().and_then(OsStr::to_str) {
                        ext = name.to_string();
                    }
                } else {
                    ProjectProgressReport::delete_by_id(&report_id)
                        .await
                        .expect("PROJECT_REPORT_DELETION_FAILED");
                    return HttpResponse::BadRequest()
                        .body("PROJECT_REPORT_DOCUMENTATION_ONLY_ACCEPTS_IMAGE".to_string());
                }
                let file_path_temp = file.file.path();
                let file_path = PathBuf::from(save_dir.to_owned() + &_id.to_string() + "." + &ext);
                if rename(file_path_temp, &file_path).is_err() {
                    if remove_dir_all(file_path).is_ok()
                        && (ProjectProgressReport::delete_by_id(&report_id).await).is_err()
                    {
                        return HttpResponse::InternalServerError()
                            .body("PROJECT_REPORT_DELETION_FAILED".to_string());
                    }
                    break;
                }
                image.extension = Some(ext.to_string());
            } else {
                ProjectProgressReport::delete_by_id(&report_id)
                    .await
                    .expect("PROJECT_REPORT_DELETION_FAILED");
                return HttpResponse::InternalServerError()
                    .body("PROJECT_REPORT_DOCUMENTATION_INVALID_LENGTH".to_string());
            }
        } else {
            ProjectProgressReport::delete_by_id(&report_id)
                .await
                .expect("PROJECT_REPORT_DELETION_FAILED");
            return HttpResponse::InternalServerError()
                .body("PROJECT_REPORT_DOCUMENTATION_MALFORMED".to_string());
        }
    }

    report.documentation = Some(documentation);

    if (report.update().await).is_err() {
        ProjectProgressReport::delete_by_id(&report_id)
            .await
            .expect("PROJECT_REPORT_DELETION_FAILED");
        HttpResponse::InternalServerError().body("PROJECT_REPORT_UPDATE_FAILED".to_string());
    }

    HttpResponse::Ok().body(report_id.to_string())
}
#[patch("/projects/{project_id}/members")]
pub async fn add_project_member(
    project_id: web::Path<String>,
    payload: web::Json<ProjectMember>,
    req: HttpRequest,
) -> HttpResponse {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::CreateRole).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    if let Ok(Some(mut project)) = Project::find_by_id(&project_id).await {
        let payload: ProjectMember = payload.into_inner();

        match project.add_member(&[payload]).await {
            Ok(project_id) => HttpResponse::Ok().body(project_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
    }
}
#[patch("/projects/{project_id}/areas")] // FINISHED
pub async fn add_project_area(
    project_id: web::Path<String>,
    payload: web::Json<ProjectAreaRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::CreateRole).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    if let Ok(Some(mut project)) = Project::find_by_id(&project_id).await {
        let payload: ProjectAreaRequest = payload.into_inner();

        match project.add_area(&[payload]).await {
            Ok(project_id) => HttpResponse::Ok().body(project_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
    }
}
