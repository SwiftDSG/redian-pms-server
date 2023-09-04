use std::{
    cmp,
    ffi::OsStr,
    fs::{self, create_dir_all, remove_dir_all, rename},
    path::{Path, PathBuf},
    vec,
};

use actix_multipart::form::MultipartForm;
use actix_web::{delete, get, post, put, web, HttpMessage, HttpRequest, HttpResponse};
use chrono::{FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use mongodb::bson::{doc, oid::ObjectId, to_bson, DateTime};
use serde::Deserialize;

use crate::models::{
    project::{
        Project, ProjectArea, ProjectAreaRequest, ProjectMemberKind, ProjectMemberRequest,
        ProjectPeriod, ProjectProgressGraphResponse, ProjectQuery, ProjectQuerySortKind,
        ProjectQueryStatusKind, ProjectRequest, ProjectStatus, ProjectStatusKind,
    },
    project_incident_report::{ProjectIncidentReport, ProjectIncidentReportRequest},
    project_progress_report::{
        ProjectProgressReport, ProjectProgressReportDocumentation,
        ProjectProgressReportDocumentationMultipartRequest, ProjectProgressReportQuery,
        ProjectProgressReportRequest,
    },
    project_role::{ProjectRole, ProjectRolePermission, ProjectRoleRequest},
    project_task::{
        ProjectTask, ProjectTaskMinResponse, ProjectTaskMultipartRequest, ProjectTaskPeriod,
        ProjectTaskPeriodRequest, ProjectTaskQuery, ProjectTaskQueryKind, ProjectTaskRequest,
        ProjectTaskStatus, ProjectTaskStatusKind, ProjectTaskStatusRequest,
        ProjectTaskTimelineQuery, ProjectTaskVolume,
    },
    role::{Role, RolePermission},
    user::UserAuthentication,
};

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTaskQueryParamsKind {
    Full,
    Default,
}
#[derive(Deserialize, Clone)]
pub struct ProjectTaskQueryParams {
    pub area_id: Option<ObjectId>,
    pub status: Option<ProjectTaskStatusKind>,
    pub kind: Option<ProjectTaskQueryParamsKind>,
}
#[derive(Deserialize)]
pub struct ProjectIncidentReportQueryParams {
    pub breakdown: bool,
}
#[derive(Deserialize)]
pub struct ProjectStatusQueryParams {
    pub status: ProjectStatusKind,
}
#[derive(Deserialize)]
pub struct ProjectProgressQueryParams {
    pub area_id: Option<ObjectId>,
}
#[derive(Deserialize)]
pub struct ProjectQueryParams {
    pub status: Option<ProjectQueryStatusKind>,
    pub sort: Option<ProjectQuerySortKind>,
    pub text: Option<String>,
    pub limit: Option<usize>,
    pub skip: Option<usize>,
}

#[get("/projects")]
pub async fn get_projects(query: web::Query<ProjectQueryParams>) -> HttpResponse {
    match Project::find_many(&ProjectQuery {
        status: query.status.clone(),
        sort: query.sort.clone(),
        text: query.text.clone(),
        limit: query.limit,
        skip: query.skip,
    })
    .await
    {
        Ok(Some(projects)) => HttpResponse::Ok().json(projects),
        Ok(None) => HttpResponse::NotFound().body("PROJECT_NOT_FOUND"),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[get("/projects/{project_id}")]
pub async fn get_project(project_id: web::Path<String>) -> HttpResponse {
    let project_id = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    match Project::find_detail_by_id(&project_id).await {
        Ok(Some(project)) => HttpResponse::Ok().json(project),
        Ok(None) => HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[get("/projects/{project_id}/areas")]
pub async fn get_project_areas(project_id: web::Path<String>) -> HttpResponse {
    let project_id: ObjectId = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    match ProjectTask::find_many_area(&project_id).await {
        Ok(Some(project)) => HttpResponse::Ok().json(project),
        Ok(None) => HttpResponse::NotFound().body("PROJECT_AREA_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[get("/projects/{project_id}/tasks")]
pub async fn get_project_tasks(
    project_id: web::Path<String>,
    query: web::Query<ProjectTaskQueryParams>,
) -> HttpResponse {
    let project_id: ObjectId = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let mut task_query = ProjectTaskTimelineQuery {
        project_id,
        area_id: query.area_id,
        task_id: None,
        status: query.status.clone(),
        relative: false,
        subtask: false,
    };

    if query.kind == Some(ProjectTaskQueryParamsKind::Full) {
        task_query.area_id = None;
        task_query.status = None;
        task_query.relative = true;
    }

    match ProjectTask::find_many_timeline(&task_query).await {
        Ok(Some(tasks)) => HttpResponse::Ok().json(tasks),
        Ok(None) => HttpResponse::Ok().json(Vec::<ProjectTaskMinResponse>::new()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[get("/projects/{project_id}/tasks/{task_id}")]
pub async fn get_project_task(_id: web::Path<(String, String)>, req: HttpRequest) -> HttpResponse {
    let (project_id, task_id) = match (_id.0.parse(), _id.1.parse()) {
        (Ok(project_id), Ok(task_id)) => (project_id, task_id),
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::GetTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    match ProjectTask::find_detail_by_id(&task_id).await {
        Ok(Some(project)) => HttpResponse::Ok().json(project),
        Ok(None) => HttpResponse::NotFound().body("PROJECT_TASK_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[get("/projects/{project_id}/progress")]
pub async fn get_project_progress(
    project_id: web::Path<String>,
    query: web::Query<ProjectProgressQueryParams>,
) -> HttpResponse {
    let project_id: ObjectId = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let mut bases: Vec<ProjectTask> = Vec::new();
    let mut dependencies: Vec<ProjectTask> = Vec::new();
    let mut progresses: Vec<ProjectProgressReport> = Vec::new();

    if let Ok(Some(tasks)) = ProjectTask::find_many(&ProjectTaskQuery {
        _id: None,
        project_id: Some(project_id),
        task_id: None,
        area_id: query.area_id,
        limit: None,
        kind: Some(ProjectTaskQueryKind::Base),
    })
    .await
    {
        bases = tasks;
    }
    if let Ok(Some(tasks)) = ProjectTask::find_many(&ProjectTaskQuery {
        _id: None,
        project_id: Some(project_id),
        task_id: None,
        area_id: query.area_id,
        limit: None,
        kind: Some(ProjectTaskQueryKind::Dependency),
    })
    .await
    {
        dependencies = tasks;
    }
    if let Ok(Some(reports)) = ProjectProgressReport::find_many(ProjectProgressReportQuery {
        project_id,
        area_id: None,
    })
    .await
    {
        progresses = reports;
    }

    if !bases.is_empty() && !dependencies.is_empty() {
        for task in bases.iter_mut() {
            let mut _id = task.task_id;
            let mut found = true;
            while found {
                if let Some(task_id) = _id {
                    if let Some(index) = dependencies.iter().position(|a| a._id.unwrap() == task_id)
                    {
                        task.value *= dependencies[index].value / 100.0;
                        _id = dependencies[index].task_id;
                    }
                } else {
                    found = false;
                }
            }
        }
    }

    let mut start_base = false;
    let mut start = 0;
    let mut end_base = false;
    let mut end = Utc::now().timestamp_millis();

    if let Some(date) = bases
        .iter()
        .filter(|a| a.period.is_some())
        .map(|a| a.period.clone().unwrap().start.timestamp_millis())
        .min()
    {
        start = date;
        start_base = true;
    }
    if let Some(date) = progresses.iter().map(|a| a.date.timestamp_millis()).min() {
        if !start_base || date < start {
            start = date;
        }
    }

    if let Some(date) = bases
        .iter()
        .filter(|a| a.period.is_some())
        .map(|a| a.period.clone().unwrap().end.timestamp_millis())
        .max()
    {
        end = date;
        end_base = true;
    }
    if let Some(date) = progresses.iter().map(|a| a.date.timestamp_millis()).max() {
        if !end_base || date > end {
            end = date
        }
    }

    let mut datas: Vec<ProjectProgressGraphResponse> = vec![ProjectProgressGraphResponse {
        x: start - 86400000,
        y: vec![0.0, 0.0],
    }];

    if start != 0 {
        let diff = (end - start) / 86400000 + 1;
        let offset = FixedOffset::east_opt(Local::now().offset().local_minus_utc()).unwrap();
        for i in 0..diff {
            let date = start + i * 86400000;
            let prev_y1 = datas.last().map_or_else(|| 0.0, |v| *v.y.first().unwrap());
            let prev_y2 = datas.last().map_or_else(|| 0.0, |v| *v.y.last().unwrap());
            let mut y1: f64 = bases
                .iter()
                .filter(|a| {
                    if let Some(period) = a.period.as_ref() {
                        let start = period.start.timestamp_millis();
                        let end = period.end.timestamp_millis();
                        date >= start && date <= end
                    } else {
                        false
                    }
                })
                .fold(prev_y1, |a, b| {
                    let period = b.period.as_ref().unwrap();
                    let start = period.start.timestamp_millis();
                    let end = period.end.timestamp_millis();
                    let diff = (end - start) / 86400000 + 1;
                    a + (b.value / (diff as f64))
                });
            let mut y2 = progresses
                .iter()
                .filter(|a| {
                    let current_date = chrono::DateTime::<Local>::from_utc(
                        NaiveDateTime::from_timestamp_opt(date / 1000, 0).unwrap(),
                        offset,
                    );
                    let progress_date = chrono::DateTime::<Local>::from_utc(
                        NaiveDateTime::from_timestamp_opt(a.date.timestamp_millis() / 1000, 0)
                            .unwrap(),
                        offset,
                    );

                    current_date.date_naive() == progress_date.date_naive()
                })
                .fold(prev_y2, |a, b| {
                    if let Some(actual) = &b.actual {
                        let progress = actual.iter().fold(0.0, |c, d| {
                            if let Some(index) =
                                bases.iter().position(|e| e._id.unwrap() == d.task_id)
                            {
                                c + d.value * bases[index].value / 100.0
                            } else {
                                c
                            }
                        });
                        a + progress
                    } else {
                        a
                    }
                });

            if y1 >= 99.99 {
                y1 = 100.0
            }
            if y2 >= 99.99 {
                y2 = 100.0
            }

            let data = ProjectProgressGraphResponse {
                x: date,
                y: vec![y1, y2],
            };

            datas.push(data);
        }
    }

    HttpResponse::Ok().json(datas)
}
#[get("/projects/{project_id}/members")]
pub async fn get_project_members(project_id: web::Path<String>) -> HttpResponse {
    let project_id: ObjectId = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    match Project::find_users(&project_id).await {
        Ok(Some(users)) => HttpResponse::Ok().json(users),
        Ok(None) => HttpResponse::NotFound().body("PROJECT_USER_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[get("/projects/{project_id}/reports")]
pub async fn get_project_reports(project_id: web::Path<String>) -> HttpResponse {
    let project_id: ObjectId = match project_id.parse() {
        Ok(project_id) => project_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    match Project::find_reports(&project_id).await {
        Ok(Some(reports)) => HttpResponse::Ok().json(reports),
        Ok(None) => HttpResponse::NotFound().body("PROJECT_REPORT_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[get("/projects/{project_id}/reports/{report_id}")]
pub async fn get_project_report(_id: web::Path<(String, String)>) -> HttpResponse {
    let report_id = match _id.1.parse() {
        Ok(report_id) => report_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    match ProjectProgressReport::find_detail_by_id(&report_id).await {
        Ok(Some(report)) => HttpResponse::Ok().json(report),
        Ok(None) => HttpResponse::NotFound().body("PROJECT_REPORT_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}

#[post("/projects")] // FINISHED
pub async fn create_project(payload: web::Json<ProjectRequest>, req: HttpRequest) -> HttpResponse {
    let issuer = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if issuer.role_id.is_empty()
        || !Role::validate(&issuer.role_id, &RolePermission::CreateProject).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let payload: ProjectRequest = payload.into_inner();

    if payload.period.start >= payload.period.end {
        return HttpResponse::BadRequest().body("INVALID_PERIOD".to_string());
    }

    let mut project: Project = Project {
        _id: None,
        customer_id: payload.customer_id,
        user_id: issuer._id.unwrap(),
        name: payload.name,
        code: payload.code,
        period: ProjectPeriod {
            start: DateTime::from_millis(payload.period.start),
            end: DateTime::from_millis(payload.period.end),
        },
        status: vec![ProjectStatus {
            kind: ProjectStatusKind::Pending,
            time: DateTime::from_millis(Utc::now().timestamp_millis()),
            message: None,
        }],
        member: None,
        area: None,
        leave: payload.leave,
        create_date: DateTime::from_millis(Utc::now().timestamp_millis()),
    };

    if let Some(_id) = payload.user_id {
        project.user_id = _id;
    }

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
                    let member = ProjectMemberRequest {
                        _id: Some(issuer._id.unwrap()),
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

#[post("/projects/{project_id}/tasks/bulk")] // FINISHED
pub async fn create_project_task_bulk(
    project_id: web::Path<String>,
    form: MultipartForm<ProjectTaskMultipartRequest>,
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

    if let Ok(Some(mut project)) = Project::find_by_id(&project_id).await {
        if project.status.first().unwrap().kind != ProjectStatusKind::Pending {
            return HttpResponse::BadRequest().body("PROJECT_STATUS_NOT_PENDING".to_string());
        }

        let path = form.file.file.path();

        if let Ok(bytes) = fs::read(path) {
            if fs::remove_file(path).is_err() {
                return HttpResponse::InternalServerError()
                    .body("PROJECT_TASK_CSV_DELETE_FAILED".to_string());
            }

            let mut row_index = -1;
            let mut data_index = 0;
            let mut data = String::new();
            let mut area_index = 0;
            let mut areas = Vec::<ProjectArea>::new();
            let mut task_level = 0;
            let mut task_value = Vec::<(usize, f64)>::new();
            let mut tasks = Vec::<ProjectTask>::new();
            let mut task: Option<ProjectTask> = None;
            let mut total = 0.0;
            for index in 1..=bytes.len() {
                let string = if index == bytes.len() {
                    String::from_utf8_lossy(&bytes[(index - 1)..])
                } else {
                    String::from_utf8_lossy(&bytes[(index - 1)..index])
                };

                if string == "\n" {
                    if let Some(mut task) = task {
                        if let Some(period) = task.period.as_mut() {
                            if let Ok(date) = NaiveDate::parse_from_str(&data, "%d-%m-%Y") {
                                period.end = DateTime::from_millis(
                                    NaiveDateTime::new(
                                        date,
                                        NaiveTime::from_hms_milli_opt(23, 59, 59, 999).unwrap(),
                                    )
                                    .timestamp_millis(),
                                )
                            }
                        }
                        tasks.push(task);
                    }
                    task = None;
                    row_index += 1;
                    data_index = 0;
                    data.clear();
                } else if string == "," || string == ";" {
                    if row_index >= 0 {
                        if data_index == 0 && !data.is_empty() {
                            if !areas.is_empty() {
                                area_index += 1;
                            }
                            areas.push(ProjectArea {
                                _id: ObjectId::new(),
                                name: data.clone(),
                            });
                        } else if data_index == 1 && !data.is_empty() {
                            let mut task_id: Option<ObjectId> = None;
                            let mut level = 0;
                            let name = data
                                .clone()
                                .chars()
                                .filter(|c| {
                                    if *c == '_' {
                                        level += 1;
                                        false
                                    } else {
                                        true
                                    }
                                })
                                .collect();

                            match level.cmp(&task_level) {
                                cmp::Ordering::Greater => task_level += 1,
                                cmp::Ordering::Less => {
                                    let mut total = 0.0;
                                    let task_complete = &task_value[level..];
                                    for (index, value) in task_complete.iter().rev() {
                                        let task_id = tasks.get(*index).unwrap()._id;
                                        for (index, task) in tasks[*index..].iter_mut().enumerate()
                                        {
                                            if index == 0 {
                                                total += *value;
                                                task.value = total;
                                            } else if task.task_id == task_id && task.value != 0.0 {
                                                task.value = task.value / total * 100.0;
                                            }
                                        }
                                    }
                                    task_value.truncate(level);
                                    task_level = level;
                                }
                                _ => (),
                            };

                            if let Some((index, _)) = task_value.last() {
                                task_id = tasks.get(*index).unwrap()._id;
                            }

                            task = Some(ProjectTask {
                                _id: Some(ObjectId::new()),
                                project_id,
                                area_id: areas.get(area_index).unwrap()._id,
                                task_id,
                                user_id: None,
                                name,
                                description: None,
                                period: None,
                                status: vec![ProjectTaskStatus {
                                    kind: ProjectTaskStatusKind::Pending,
                                    time: DateTime::from_millis(Utc::now().timestamp_millis()),
                                    message: None,
                                }],
                                volume: None,
                                value: 0.0,
                            });
                        } else if data_index == 2 && !data.is_empty() {
                            if let Some(task) = task.as_mut() {
                                if let Ok(value) = data.parse::<usize>() {
                                    task.volume = Some(ProjectTaskVolume {
                                        value,
                                        unit: "pcs".to_owned(),
                                    });
                                }
                            }
                        } else if data_index == 3 && !data.is_empty() {
                            if let Some(task) = task.as_mut() {
                                if let Some(volume) = task.volume.as_mut() {
                                    volume.unit = data.clone();
                                }
                            }
                        } else if data_index == 4 {
                            if let Some(task) = task.as_mut() {
                                if let Ok(value) = data.parse::<f64>() {
                                    if let Some(parent_value) = task_value.last_mut() {
                                        parent_value.1 =
                                            ((parent_value.1 + value) * 1e6_f64).round() / 1e6_f64;
                                    }
                                    total = ((total + value) * 1e6_f64).round() / 1e6_f64;
                                    task.value = value;
                                } else {
                                    task_value.push((tasks.len(), 0.0));
                                    task.value = 0.0;
                                }
                            }
                        } else if data_index == 5 && !data.is_empty() {
                            if let Some(task) = task.as_mut() {
                                if let Ok(date) = NaiveDate::parse_from_str(&data, "%d-%m-%Y") {
                                    task.period = Some(ProjectTaskPeriod {
                                        start: DateTime::from_millis(
                                            NaiveDateTime::new(
                                                date,
                                                NaiveTime::from_hms_milli_opt(0, 0, 0, 0).unwrap(),
                                            )
                                            .timestamp_millis(),
                                        ),
                                        end: DateTime::from_millis(
                                            NaiveDateTime::new(
                                                date,
                                                NaiveTime::from_hms_milli_opt(23, 59, 59, 999)
                                                    .unwrap(),
                                            )
                                            .timestamp_millis(),
                                        ),
                                    });
                                }
                            }
                        }
                    }

                    data_index += 1;
                    data.clear();
                } else {
                    data.push_str(&string);
                }
            }

            if (total - 100.0).abs() > 0.001 {
                return HttpResponse::BadRequest().body("PROJECT_TASK_INVALID_VALUE");
            }

            if let Some(mut task) = task {
                if let Some(period) = task.period.as_mut() {
                    if let Ok(date) = NaiveDate::parse_from_str(&data, "%d-%m-%Y") {
                        period.end = DateTime::from_millis(
                            NaiveDateTime::new(
                                date,
                                NaiveTime::from_hms_milli_opt(23, 59, 59, 999).unwrap(),
                            )
                            .timestamp_millis(),
                        )
                    }
                }
                tasks.push(task);
            }

            let mut total = 0.0;
            for (index, value) in task_value.iter().rev() {
                let task_id = tasks.get(*index).unwrap()._id;
                for (index, task) in tasks[*index..].iter_mut().enumerate() {
                    if index == 0 {
                        total += *value;
                        task.value = total;
                    } else if task.task_id == task_id && task.value != 0.0 {
                        task.value = task.value / total * 100.0;
                    }
                }
            }

            if ProjectTask::delete_many_by_project_id(&project_id)
                .await
                .is_err()
            {
                return HttpResponse::InternalServerError().body("PROJECT_TASK_DELETE_FAILED");
            }
            if project.replace_areas(areas).await.is_err() {
                return HttpResponse::InternalServerError().body("PROJECT_AREA_CREATION_FAILED");
            }
            match ProjectTask::save_bulk(tasks).await {
                Ok(task_id) => HttpResponse::Created().json(doc! {
                    "_id": to_bson::<Vec<ObjectId>>(&task_id).unwrap()
                }),
                Err(error) => HttpResponse::InternalServerError().body(error),
            }
        } else {
            HttpResponse::BadRequest().body("PROJECT_TASK_CSV_UPLOAD_FAILED")
        }
    } else {
        HttpResponse::BadRequest().body("PROJECT_TASK_CSV_UPLOAD_FAILED")
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
        Ok(task_id) => HttpResponse::Created().body(task_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/projects/{project_id}/tasks/{task_id}")] // FINISHED
pub async fn create_project_task_sub(
    _id: web::Path<(String, String)>,
    payload: web::Json<Vec<ProjectTaskRequest>>,
    req: HttpRequest,
) -> HttpResponse {
    let (project_id, task_id) = match (_id.0.parse(), _id.1.parse::<ObjectId>()) {
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

    if let Ok(Some(_)) = ProjectTask::find_many(&ProjectTaskQuery {
        _id: None,
        project_id: None,
        task_id: Some(task_id),
        area_id: None,
        limit: None,
        kind: None,
    })
    .await
    {
        if ProjectTask::delete_many_by_task_id(&task_id).await.is_err() {
            return HttpResponse::InternalServerError()
                .body("PROJECT_TASK_DELETION_FAILED".to_string());
        }
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

            HttpResponse::Created().json(doc! {
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
        user_id: issuer_id,
        date: DateTime::from_millis(Utc::now().timestamp_millis()),
        time: payload.time,
        member_id: payload.member_id,
        actual: payload.actual,
        plan: payload.plan,
        documentation: None,
        weather: payload.weather,
    };

    if let Some(documentation) = payload.documentation {
        let docs: Vec<ProjectProgressReportDocumentation> = documentation
            .iter()
            .map(|a| ProjectProgressReportDocumentation {
                description: a.description.clone(),
                extension: a.extension.clone(),
                _id: ObjectId::new(),
            })
            .collect();
        project_report.documentation = Some(docs);
    }

    match project_report.save().await {
        Ok(report_id) => HttpResponse::Created().body(report_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}

#[post("/projects/{project_id}/incidents")]
pub async fn create_project_incident(
    project_id: web::Path<String>,
    payload: web::Json<ProjectIncidentReportRequest>,
    query: web::Query<ProjectIncidentReportQueryParams>,
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
        user_id: issuer_id,
        member_id: payload.member_id,
        kind: payload.kind,
        date: DateTime::from_millis(Utc::now().timestamp_millis()),
    };

    match project_incident.save(query.breakdown).await {
        Ok(incident_id) => HttpResponse::Created().body(incident_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}

#[put("/projects/{project_id}/status")]
pub async fn update_project_status(
    _id: web::Path<String>,
    query: web::Query<ProjectStatusQueryParams>,
    req: HttpRequest,
) -> HttpResponse {
    let project_id = match _id.parse() {
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

    if let Ok(Some(mut project)) = Project::find_by_id(&project_id).await {
        if query.status != ProjectStatusKind::Running {
            return HttpResponse::BadRequest().body("INVALID_STATUS".to_string());
        }

        if project.status.first().unwrap().kind != ProjectStatusKind::Breakdown
            && project.status.first().unwrap().kind != ProjectStatusKind::Paused
        {
            return HttpResponse::BadRequest().body("PROJECT_STATUS_INVALID".to_string());
        }

        match project.update_status(query.status.clone(), None).await {
            Ok(project_id) => HttpResponse::Ok().body(project_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
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
            task.user_id = payload.user_id;

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
#[put("/projects/{project_id}/tasks/{task_id}/status")]
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
#[put("/projects/{project_id}/tasks/{task_id}/period")]
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
            start: DateTime::from_millis(payload.start),
            end: DateTime::from_millis(payload.end),
        };

        match task.update_period(period).await {
            Ok(task_id) => HttpResponse::Ok().body(task_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_TASK_NOT_FOUND".to_string())
    }
}
#[put("/projects/{project_id}/reports/{report_id}")] // REDO ALL CHANGES WHEN FAILED
pub async fn update_project_report(
    _id: web::Path<(String, String)>,
    form: MultipartForm<ProjectProgressReportDocumentationMultipartRequest>,
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
        if let Some(image) = documentation.get_mut(i) {
            let mut ext = String::new();
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
            let file_path =
                PathBuf::from(save_dir.to_owned() + &image._id.to_string() + "." + &ext);
            if rename(file_path_temp, &file_path).is_err() {
                if remove_dir_all(file_path).is_ok()
                    && (ProjectProgressReport::delete_by_id(&report_id).await).is_err()
                {
                    return HttpResponse::InternalServerError()
                        .body("PROJECT_REPORT_DELETION_FAILED".to_string());
                }
                break;
            }
            image.extension = ext.to_string();
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
#[put("/projects/{project_id}/roles/{role_id}")] // REDO ALL CHANGES WHEN FAILED
pub async fn update_project_role(
    _id: web::Path<(String, String)>,
    payload: web::Json<ProjectRoleRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let (project_id, role_id) = match (_id.0.parse(), _id.1.parse()) {
        (Ok(project_id), Ok(role_id)) => (project_id, role_id),
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::UpdateRole).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let mut project_role = match ProjectRole::find_by_id(&role_id).await {
        Ok(Some(role)) => role,
        Ok(None) => return HttpResponse::NotFound().body("PROJECT_ROLE_NOT_FOUND"),
        Err(_) => return HttpResponse::NotFound().body("PROJECT_ROLE_NOT_FOUND"),
    };

    let payload: ProjectRoleRequest = payload.into_inner();

    project_role.name = payload.name;
    project_role.permission = payload.permission;

    match project_role.update().await {
        Ok(role_id) => HttpResponse::Ok().body(role_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[put("/projects/{project_id}/members")]
pub async fn add_project_member(
    project_id: web::Path<String>,
    payload: web::Json<ProjectMemberRequest>,
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
        let payload: ProjectMemberRequest = payload.into_inner();

        match project.add_member(&[payload]).await {
            Ok(project_id) => HttpResponse::Ok().body(project_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
    }
}
//DIGANTI POST -> PATCH!!!!!
#[put("/projects/{project_id}/areas")] // FINISHED
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
#[delete("/projects/{project_id}/areas/{area_id}")]
pub async fn delete_project_area(
    _id: web::Path<(String, String)>,
    req: HttpRequest,
) -> HttpResponse {
    let (project_id, area_id) = match (_id.0.parse(), _id.1.parse()) {
        (Ok(project_id), Ok(area_id)) => (project_id, area_id),
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_id = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer._id.unwrap(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::DeleteTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    if let Ok(Some(mut project)) = Project::find_by_id(&project_id).await {
        if ProjectTask::delete_many_by_area_id(&area_id).await.is_ok() {
            match project.remove_area(&area_id).await {
                Ok(_id) => HttpResponse::Ok().body(_id.to_string()),
                Err(error) => HttpResponse::InternalServerError().body(error),
            }
        } else {
            HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
    }
}
#[delete("/projects/{project_id}/tasks/{task_id}")]
pub async fn delete_project_task(
    _id: web::Path<(String, String)>,
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
    if !ProjectRole::validate(&project_id, &issuer_id, &ProjectRolePermission::DeleteTask).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    if let Ok(Some(_)) = Project::find_by_id(&project_id).await {
        match ProjectTask::delete_by_id(&task_id).await {
            Ok(result) => HttpResponse::NoContent().body(result.to_string()),
            Err(_) => HttpResponse::NotFound().body("PROJECT_TASK_NOT_FOUND".to_string()),
        }
    } else {
        HttpResponse::NotFound().body("PROJECT_NOT_FOUND".to_string())
    }
}
