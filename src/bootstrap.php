<?php

use \Symfony\Component\Yaml\Yaml;
use \Silex\Provider;
use \PommProject\Silex\ {
    ServiceProvider\PommServiceProvider,
    ProfilerServiceProvider\PommProfilerServiceProvider
};

require_once __DIR__ . '/../vendor/autoload.php';

$app = new Silex\Application();

$app['config'] = function () {
    if (!is_file(__DIR__ . '/config/parameters.yml')) {
        throw new \RunTimeException('No current configuration file found in config.');
    }

    $config = Yaml::parse(file_get_contents(__DIR__ . '/config/parameters.yml'));
    $parameters = $config['parameters'];

    $parameters['pomm'] = [
        $parameters['database_name'] => [
            'class:session_builder' => '\PommProject\ModelManager\SessionBuilder',
            'dsn' => sprintf(
                "pgsql://%s:%s@%s:%s/%s",
                $parameters['database_user'],
                $parameters['database_password'],
                $parameters['database_host'],
                $parameters['database_port'],
                $parameters['database_name']
            ),
        ],
    ];

    return $parameters;
};

$app['debug'] = function () {
    return getenv('APP_DEBUG') !== 0 && getenv('APP_ENVIRONMENT') !== 'prod';
};

$app->register(new Provider\SessionServiceProvider);

$app->register(new Provider\TwigServiceProvider, array(
    'twig.path' => __DIR__ . '/views',
));

$app->register(new PommServiceProvider(), array(
    'pomm.configuration' => $app['config']['pomm'],
));

$app['db'] = function () use($app) {
    return $app['pomm']['sav'];
};

if (class_exists('\Silex\Provider\WebProfilerServiceProvider')) {
    $app->register(new Provider\ServiceControllerServiceProvider);
    $app->register(new Provider\HttpFragmentServiceProvider);

    $profiler = new Provider\WebProfilerServiceProvider();
    $app->register($profiler, array(
        'profiler.cache_dir' => __DIR__ . '/../cache/profiler',
        'profiler.mount_prefix' => '/_profiler',
    ));

    $app->register(new PommProfilerServiceProvider);
}

return $app;
