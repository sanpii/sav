<?php
declare(strict_types = 1);

namespace App\Controller;

use \PommProject\Foundation\Pomm;
use \PommProject\Foundation\Where;
use \Symfony\Component\HttpFoundation\Request;
use \Symfony\Component\HttpFoundation\Response;
use \Symfony\Component\Templating\EngineInterface;

class Index
{
    private $pomm;
    private $templating;

    public function __construct(EngineInterface $templating, Pomm $pomm)
    {
        $this->templating = $templating;
        $this->pomm = $pomm;
    }

    public function index(Request $request): Response
    {
        $page = $request->get('page', 1);
        $trashed = $request->get('trashed', false);
        $limit = $request->get('limit', 50);

        if ($trashed) {
            $where = new Where('trashed_at IS NOT NULL');
        }
        else {
            $where = new Where('trashed_at IS NULL');
        }

        $pager = $this->pomm['db']->getModel('\App\Model\ExpenseModel')
            ->paginateFindWhere(
                $where,
                $limit,
                $page,
                'ORDER BY created_at DESC'
            );

        return new Response(
            $this->templating->render(
                'expense/list.html.twig',
                compact('pager')
            )
        );
    }
}
