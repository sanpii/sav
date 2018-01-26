<?php

namespace App\Model;

use PommProject\ModelManager\Model\Model;
use PommProject\ModelManager\Model\Projection;
use PommProject\ModelManager\Model\ModelTrait\WriteQueries;

use PommProject\Foundation\Where;

use App\Model\AutoStructure\Expense as ExpenseStructure;
use App\Model\Expense;

/**
 * ExpenseModel
 *
 * Model class for table expense.
 *
 * @see Model
 */
class ExpenseModel extends Model
{
    use WriteQueries;

    /**
     * __construct()
     *
     * Model constructor
     *
     * @access public
     * @return void
     */
    public function __construct()
    {
        $this->structure = new ExpenseStructure;
        $this->flexible_entity_class = '\App\Model\Expense';
    }

    public function createProjection()
    {
        return parent::createProjection()
            ->setField('trashed', 'trashed_at is not null', 'boolean')
            ->setField('warranty_at', 'created_at + warranty', 'date')
            ->setField('warranty_active', 'created_at + warranty > now()', 'boolean');
    }
}
