//! Apache Airflow DAG definition file (Python) detection and census.
//!
//! Detects `from airflow`/`import airflow` imports with DAG usage
//! (`DAG(`, `dag_id`, `with DAG`, `@dag`) and counts `dag_id` kwargs,
//! `@task`/`@dag`/`@task_group` decorators, built-in operator call sites
//! (`BashOperator(`, `PythonOperator(`, sensors, `TriggerDagRunOperator(`,
//! transfer/container operators), `task_id=` kwargs, `>>`/`<<` dependency
//! operators, settings kwargs (`schedule`/`schedule_interval`/`default_args`/
//! `catchup`/`tags`/`params`/`max_active_*`/`retries`/`retry_delay`/`start_date`/
//! `end_date`/`concurrency`/`depends_on_past`/`wait_for_downstream`/
//! `on_*_callback`/`sla`/`execution_timeout`/`doc_md`/`description`), and
//! `#` comment lines.
//!
//! ```
//! let b = b"from airflow import DAG\nfrom airflow.operators.bash import BashOperator\n\nwith DAG(dag_id=\"demo\", schedule=\"@daily\", catchup=False):\n    t1 = BashOperator(task_id=\"t1\", bash_command=\"echo x\")\n    t2 = BashOperator(task_id=\"t2\", bash_command=\"echo y\")\n    t1 >> t2\n";
//! assert!(izanagi_kit::airflow::detect(b));
//! let c = izanagi_kit::airflow::Airflow::parse(b).unwrap();
//! assert_eq!(c.imports, 2);
//! assert_eq!(c.operators, 2);
//! assert_eq!(c.deps, 1);
//! ```

/// Parsed Airflow DAG definition summary.
#[derive(Debug, Clone)]
pub struct Airflow {
    /// `from airflow`/`import airflow` import statement lines.
    pub imports: usize,
    /// `dag_id` occurrences (kwargs and context keys).
    pub dag_ids: usize,
    /// `@task`/`@dag`/`@task_group`/`@task.branch`/`@task.virtualenv` decorators.
    pub decorators: usize,
    /// known operator call sites (`XOperator(`/sensor/`TriggerDagRunOperator(`).
    pub operators: usize,
    /// `task_id=` keyword arguments.
    pub tasks: usize,
    /// `>>`/`<<` dependency operators.
    pub deps: usize,
    /// settings kwargs (`schedule`/`catchup`/`tags`/`params`/`retries`/…).
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const OPERATORS: &[&str] = &[
    "BashOperator(",
    "PythonOperator(",
    "BranchPythonOperator(",
    "ShortCircuitOperator(",
    "EmptyOperator(",
    "DummyOperator(",
    "BranchSQLOperator(",
    "SqlOperator(",
    "PostgresOperator(",
    "MySqlOperator(",
    "MySQLOperator(",
    "SqliteOperator(",
    "SQLiteOperator(",
    "JdbcOperator(",
    "OracleOperator(",
    "MSSQLOperator(",
    "KubernetesPodOperator(",
    "DockerOperator(",
    "DockerSwarmOperator(",
    "PodOperator(",
    "EmailOperator(",
    "HttpOperator(",
    "SimpleHttpOperator(",
    "SparkSubmitOperator(",
    "SparkSqlOperator(",
    "SparkJDBCOperator(",
    "TriggerDagRunOperator(",
    "SubDagOperator(",
    "LatestOnlyOperator(",
    "WeekdayBranchOperator(",
    "WeekDayBranchOperator(",
    "BranchDateTimeOperator(",
    "BranchDayOfWeekOperator(",
    "SnowflakeOperator(",
    "BigQueryOperator(",
    "BigQueryInsertJobOperator(",
    "BigQueryExecuteQueryOperator(",
    "GCSToGCSOperator(",
    "GCSToS3Operator(",
    "S3ToGCSOperator(",
    "S3DeleteObjectsOperator(",
    "S3FileTransformOperator(",
    "RedshiftSQLOperator(",
    "AthenaOperator(",
    "HiveOperator(",
    "HiveToMySqlTransfer(",
    "MySqlToHiveTransfer(",
    "PrestoToMySqlTransfer(",
    "DbtRunOperator(",
    "KubernetesPodDecorator(",
    "Sensor(",
    "SqlSensor(",
    "ExternalTaskSensor(",
    "FileSensor(",
    "S3KeySensor(",
    "DateTimeSensor(",
    "DateTimeSensorAsync(",
    "TimeDeltaSensor(",
    "TimeDeltaSensorAsync(",
    "TimeSensor(",
    "TimeSensorAsync(",
    "HttpSensor(",
    "PythonSensor(",
    "BashSensor(",
    "HivePartitionSensor(",
    "NamedHivePartitionSensor(",
    "MetastorePartitionSensor(",
    "DagRunSensor(",
];

const SETTINGS: &[&str] = &[
    "schedule=",
    "schedule_interval=",
    "timetable=",
    "default_args",
    "catchup=",
    "tags=",
    "params=",
    "max_active_runs",
    "max_active_tasks",
    "retries",
    "retry_delay",
    "retry_exponential_backoff",
    "max_retry_delay",
    "start_date",
    "end_date",
    "concurrency=",
    "depends_on_past",
    "wait_for_downstream",
    "on_failure_callback",
    "on_success_callback",
    "on_retry_callback",
    "on_skipped_callback",
    "sla=",
    "sla_miss_callback",
    "execution_timeout",
    "doc_md",
    "description=",
    "default_view",
    "orientation=",
    "dagrun_timeout",
    "is_paused_upon_creation",
    "render_template_as_native_obj",
    "template_searchpath",
    "template_undefined",
    "user_defined_macros",
    "user_defined_filters",
    "access_control",
    "owner=",
    "pool=",
    "pool_slots",
    "priority_weight",
    "queue=",
    "run_as_user",
    "execution_date",
    "execution_delta",
    "execution_group",
    "trigger_rule",
    "weight_rule",
    "executor=",
    "executor_config",
    "do_xcom_push",
    "xcom_push",
    "group_id",
    "task_group",
    "mapped",
    "inlets",
    "outlets",
    "resources",
    "env=",
    "bash_command",
    "python_callable",
    "op_args",
    "op_kwargs",
    "templates_dict",
    "provide_context",
    "namespace=",
    "image=",
    "cmds=",
    "arguments=",
    "labels=",
    "startup_timeout_seconds",
    "get_logs",
    "image_pull_policy",
    "hostnetwork",
    "volumes=",
    "volume_mounts",
    "pod_template_file",
    "log_events_on_failure",
    "secrets=",
    "reattach_on_restart",
    "is_delete_operator_pod",
    "termination_grace_period",
    "sql=",
    "sql_args",
    "conn_id",
    "autocommit",
    "parameters=",
    "handler",
    "split_statements",
    "return_last",
    "database=",
    "schema=",
    "endpoint=",
    "data=",
    "headers=",
    "response_check",
    "response_filter",
    "extra_options",
    "http_conn_id",
    "method=",
    "log_response",
    "use_ssl",
    "verify=",
    "proxies",
    "cert=",
];
fn code_has(t: &str, needle: &str) -> bool {
    // `#` コメント行内の言及は証拠にしない。
    t.lines()
        .any(|l| !l.trim_start().starts_with('#') && l.contains(needle))
}

/// Detects Airflow DAG Python source files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    (code_has(&t, "from airflow") || code_has(&t, "import airflow"))
        && (code_has(&t, "DAG(")
            || code_has(&t, "dag_id")
            || code_has(&t, "with DAG")
            || code_has(&t, "@dag"))
}

fn count_lines<F: Fn(&str) -> bool>(t: &str, f: F) -> usize {
    t.lines().map(str::trim).filter(|l| f(l)).count()
}

impl Airflow {
    /// Parses an Airflow DAG file, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let imports = count_lines(&t, |l| {
            l.starts_with("from airflow") || l.starts_with("import airflow")
        });
        let decorators = count_lines(&t, |l| {
            l.starts_with("@task") || l.starts_with("@dag") || l.starts_with("@task_group")
        });
        let operators = OPERATORS.iter().map(|o| t.matches(o).count()).sum();
        let settings = SETTINGS.iter().map(|s| t.matches(s).count()).sum();
        let deps = t.matches(">>").count() + t.matches("<<").count();
        Some(Self {
            imports,
            dag_ids: t.matches("dag_id").count(),
            decorators,
            operators,
            tasks: t.matches("task_id=").count(),
            deps,
            settings,
            comments: count_lines(&t, |l| l.starts_with('#')),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# from airflow import DAG\n# with DAG() as d:\n"));
    }

    #[test]
    fn detects_and_counts() {
        let b = b"from airflow import DAG\nfrom airflow.operators.bash import BashOperator\n\nwith DAG(dag_id=\"demo\", schedule=\"@daily\", catchup=False, tags=[\"x\"]):\n    t1 = BashOperator(task_id=\"t1\", bash_command=\"echo x\")\n    t2 = BashOperator(task_id=\"t2\", bash_command=\"echo y\")\n    t1 >> t2\n";
        let c = Airflow::parse(b).unwrap();
        assert_eq!(c.imports, 2);
        assert_eq!(c.dag_ids, 1);
        assert_eq!(c.operators, 2);
        assert_eq!(c.tasks, 2);
        assert_eq!(c.deps, 1);
        assert!(c.settings >= 3);
    }

    #[test]
    fn counts_taskflow_decorators() {
        let b = b"from airflow.decorators import dag, task\n\n@dag(dag_id=\"tf\", schedule=None)\ndef pipeline():\n    @task\n    def extract():\n        return 1\n    extract()\n";
        let c = Airflow::parse(b).unwrap();
        assert_eq!(c.decorators, 2);
        assert_eq!(c.tasks, 0);
    }

    #[test]
    fn rejects_plain_python() {
        assert!(Airflow::parse(b"def f():\n    return 1\n").is_none());
        assert!(!detect(b"import os\nwith open(\"x\"):\n    pass\n"));
    }
}
